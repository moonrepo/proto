mod detect;
mod download;
mod execute;
mod host_funcs;
mod install;
mod resolve;
mod shim;
mod verify;

use extism::{manifest::Wasm, Manifest as PluginManifest, Plugin};
use host_funcs::HostData;
use once_cell::sync::OnceCell;
use once_map::OnceMap;
use proto_core::{impl_tool, Describable, Manifest, Proto, ProtoError, Resolvable, Tool};
use proto_pdk_api::{
    DownloadPrebuiltInput, DownloadPrebuiltOutput, EmptyInput, Environment, HostArch, HostOS,
    ToolMetadataInput, ToolMetadataOutput,
};
use rustc_hash::FxHashMap;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    any::Any,
    env::{self, consts},
    fmt::Debug,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, RwLock},
};
use tracing::trace;

pub struct WasmPlugin {
    pub id: String,
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub shim_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,

    manifest: OnceCell<Manifest>,
    plugin: Arc<RwLock<Plugin<'static>>>,
    plugin_paths: FxHashMap<PathBuf, PathBuf>,
    func_cache: OnceMap<String, Vec<u8>>,
}

impl WasmPlugin {
    pub fn new<P: AsRef<Proto>, L: AsRef<Path>>(
        proto: P,
        id: String,
        wasm_file: L,
    ) -> Result<Self, ProtoError> {
        let proto = proto.as_ref();
        let working_dir = env::current_dir().unwrap();
        let plugin_paths = FxHashMap::from_iter([
            (PathBuf::from("/home"), proto.home_dir.clone()),
            (PathBuf::from("/proto"), proto.root.clone()),
            (PathBuf::from("/workspace"), working_dir.clone()),
        ]);

        let mut manifest = PluginManifest::new([Wasm::file(wasm_file)]);
        manifest = manifest.with_allowed_host("*");

        for (virtual_path, real_path) in &plugin_paths {
            manifest = manifest.with_allowed_path(real_path, virtual_path);
        }

        let host_data = HostData { working_dir };
        let plugin =
            Plugin::create_with_manifest(&manifest, host_funcs::create_functions(host_data), true)
                .map_err(|e| ProtoError::PluginWasmCreateFailed(e.to_string()))?;

        let wasm_plugin = WasmPlugin {
            base_dir: proto.tools_dir.join(&id),
            bin_path: None,
            manifest: OnceCell::new(),
            shim_path: None,
            temp_dir: proto.temp_dir.join(&id),
            version: None,
            id,
            plugin: Arc::new(RwLock::new(plugin)),
            plugin_paths,
            func_cache: OnceMap::new(),
        };

        // Load metadata on load and make available
        wasm_plugin.get_metadata()?;

        Ok(wasm_plugin)
    }

    fn get_environment(&self) -> Result<Environment, ProtoError> {
        let version = self.get_resolved_version();

        let env = self
            .func_cache
            .try_insert_cloned(format!("env-{version}"), |_| {
                let env = Environment {
                    arch: HostArch::from_str(consts::ARCH)
                        .map_err(|e| ProtoError::Message(e.to_string()))?,
                    os: HostOS::from_str(consts::OS)
                        .map_err(|e| ProtoError::Message(e.to_string()))?,
                    vars: self
                        .get_metadata()?
                        .env_vars
                        .iter()
                        .filter_map(|var| env::var(var).ok().map(|value| (var.to_owned(), value)))
                        .collect(),
                    version: version.to_owned(),
                };

                Ok::<Vec<u8>, ProtoError>(self.format_input(env)?.as_bytes().to_vec())
            })?;

        self.parse_output(&env)
    }

    fn get_install_params(&self) -> Result<DownloadPrebuiltOutput, ProtoError> {
        self.cache_func_with(
            "download_prebuilt",
            DownloadPrebuiltInput {
                env: self.get_environment()?,
            },
        )
    }

    fn get_metadata(&self) -> Result<ToolMetadataOutput, ProtoError> {
        self.cache_func_with(
            "register_tool",
            ToolMetadataInput {
                id: self.get_id().to_owned(),
                env: Environment {
                    arch: HostArch::from_str(consts::ARCH)
                        .map_err(|e| ProtoError::Message(e.to_string()))?,
                    os: HostOS::from_str(consts::OS)
                        .map_err(|e| ProtoError::Message(e.to_string()))?,
                    ..Environment::default()
                },
            },
        )
    }

    pub fn to_wasi_virtual_path(&self, path: &Path) -> PathBuf {
        for (virtual_path, real_path) in &self.plugin_paths {
            if path.starts_with(real_path) {
                let path = virtual_path.join(path.strip_prefix(real_path).unwrap());

                // Only forward slashes are allowed in WASI
                return if cfg!(windows) {
                    PathBuf::from(path.to_string_lossy().replace('\\', "/"))
                } else {
                    path
                };
            }
        }

        path.to_owned()
    }
}

impl Describable<'_> for WasmPlugin {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> String {
        self.get_metadata().unwrap().name
    }
}

impl_tool!(WasmPlugin);

impl WasmPlugin {
    fn call(&self, func: &str, input: impl AsRef<[u8]>) -> Result<&[u8], ProtoError> {
        let input = input.as_ref();

        trace!(
            tool = self.get_id(),
            func,
            input = %String::from_utf8_lossy(input),
            "Calling function on plugin"
        );

        let output = self
            .plugin
            .write()
            .expect("Failed to get write access to WASM plugin!")
            .call(func, input)
            .map_err(|e| ProtoError::PluginWasmCallFailed(e.to_string()))?;

        if !output.is_empty() {
            trace!(
                tool = self.get_id(),
                func,
                output = %String::from_utf8_lossy(output),
                "Received function response"
            );
        }

        Ok(output)
    }

    fn format_input<I: Serialize>(&self, input: I) -> Result<String, ProtoError> {
        serde_json::to_string(&input).map_err(|e| ProtoError::PluginWasmCallFailed(e.to_string()))
    }

    fn parse_output<O: DeserializeOwned>(&self, data: &[u8]) -> Result<O, ProtoError> {
        serde_json::from_slice(data).map_err(|e| ProtoError::PluginWasmCallFailed(e.to_string()))
    }

    pub fn cache_func<O>(&self, func: &str) -> Result<O, ProtoError>
    where
        O: Debug + DeserializeOwned,
    {
        self.cache_func_with(func, EmptyInput::default())
    }

    pub fn cache_func_with<I, O>(&self, func: &str, input: I) -> Result<O, ProtoError>
    where
        I: Debug + Serialize,
        O: Debug + DeserializeOwned,
    {
        let input = self.format_input(input)?;
        let cache_key = format!("{func}-{input}");

        // Check if cache exists already in read-only mode
        {
            if let Some(data) = self.func_cache.get(&cache_key) {
                return self.parse_output(data);
            }
        }

        // Otherwise call the function and cache the result
        let data = self.call(func, input)?;
        let output: O = self.parse_output(data)?;

        self.func_cache.insert(cache_key, |_| data.to_vec());

        Ok(output)
    }

    pub fn call_func<O>(&self, func: &str) -> Result<O, ProtoError>
    where
        O: Debug + DeserializeOwned,
    {
        self.call_func_with(func, EmptyInput::default())
    }

    pub fn call_func_with<I, O>(&self, func: &str, input: I) -> Result<O, ProtoError>
    where
        I: Debug + Serialize,
        O: Debug + DeserializeOwned,
    {
        self.parse_output(self.call(func, self.format_input(input)?)?)
    }

    pub fn has_func(&self, func: &str) -> bool {
        self.plugin.read().unwrap().has_function(func)
    }
}
