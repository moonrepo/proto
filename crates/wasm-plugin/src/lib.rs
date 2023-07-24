mod detect;
mod download;
mod execute;
pub mod host_funcs;
mod install;
mod resolve;
mod shim;
mod verify;

use extism::{manifest::Wasm, Manifest as PluginManifest};
use host_funcs::HostData;
use once_cell::sync::OnceCell;
use proto_core::{impl_tool, Describable, Manifest, Proto, ProtoError, Resolvable, Tool};
use proto_pdk_api::{
    DownloadPrebuiltInput, DownloadPrebuiltOutput, Environment, HostArch, HostOS,
    ToolMetadataInput, ToolMetadataOutput,
};
use rustc_hash::FxHashMap;
use std::{
    any::Any,
    env::{self, consts},
    path::{Path, PathBuf},
    str::FromStr,
};
use warpgate::PluginContainer;

pub struct WasmPlugin {
    pub id: String,
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub shim_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,

    pub container: PluginContainer<'static>,
    manifest: OnceCell<Manifest>,
    proto: Proto,
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
            (PathBuf::from("/workspace"), working_dir.clone()),
            (PathBuf::from("/home"), proto.home.clone()),
            (PathBuf::from("/proto"), proto.root.clone()),
        ]);

        let mut manifest = PluginManifest::new([Wasm::file(wasm_file)]);
        manifest = manifest.with_allowed_host("*");

        for (virtual_path, real_path) in &plugin_paths {
            manifest = manifest.with_allowed_path(real_path, virtual_path);
        }

        let host_data = HostData { working_dir };

        let wasm_plugin = WasmPlugin {
            base_dir: proto.tools_dir.join(&id),
            bin_path: None,
            container: PluginContainer::new(&id, manifest, host_funcs::create_functions(host_data))
                .map_err(|e| ProtoError::Message(e.to_string()))?,
            manifest: OnceCell::new(),
            shim_path: None,
            temp_dir: proto.temp_dir.join(&id),
            version: None,
            id,
            proto: proto.to_owned(),
        };

        // Load metadata on load and make available
        wasm_plugin.get_metadata()?;

        Ok(wasm_plugin)
    }

    pub fn get_environment(&self) -> Result<Environment, ProtoError> {
        Ok(Environment {
            arch: HostArch::from_str(consts::ARCH)
                .map_err(|e| ProtoError::Message(e.to_string()))?,
            id: self.id.clone(),
            os: HostOS::from_str(consts::OS).map_err(|e| ProtoError::Message(e.to_string()))?,
            vars: self
                .get_metadata()?
                .env_vars
                .iter()
                .filter_map(|var| env::var(var).ok().map(|value| (var.to_owned(), value)))
                .collect(),
            version: self.get_resolved_version().to_owned(),
        })
    }

    pub fn get_install_params(&self) -> Result<DownloadPrebuiltOutput, ProtoError> {
        self.container
            .cache_func_with(
                "download_prebuilt",
                DownloadPrebuiltInput {
                    env: self.get_environment()?,
                },
            )
            .map_err(|e| ProtoError::Message(e.to_string()))
    }

    pub fn get_metadata(&self) -> Result<ToolMetadataOutput, ProtoError> {
        self.container
            .cache_func_with(
                "register_tool",
                ToolMetadataInput {
                    id: self.id.clone(),
                    env: Environment {
                        arch: HostArch::from_str(consts::ARCH)
                            .map_err(|e| ProtoError::Message(e.to_string()))?,
                        id: self.id.clone(),
                        os: HostOS::from_str(consts::OS)
                            .map_err(|e| ProtoError::Message(e.to_string()))?,
                        ..Environment::default()
                    },
                },
            )
            .map_err(|e| ProtoError::Message(e.to_string()))
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
