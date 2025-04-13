use crate::endpoints::Empty;
use crate::helpers::{from_virtual_path, to_virtual_path};
use crate::id::Id;
use crate::plugin_error::WarpgatePluginError;
use extism::{Error, Function, Manifest, Plugin};
use miette::IntoDiagnostic;
use serde::Serialize;
use serde::de::DeserializeOwned;
use starbase_styles::{apply_style_tags, color};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use system_env::{SystemArch, SystemLibc, SystemOS};
use tokio::sync::RwLock;
use tokio::task::block_in_place;
use tracing::{instrument, trace};
use warpgate_api::{HostEnvironment, VirtualPath};

fn is_incompatible_runtime(error: &Error) -> bool {
    let check = |message: String| {
        // unknown import: `env::exec_command` has not been defined
        message.contains("unknown import") && message.contains("env::")
    };

    if let Some(source) = error.source() {
        if check(source.to_string()) {
            return true;
        }
    }

    check(error.to_string())
}

// Compatibility with extism >= 1.6
fn map_virtual_paths(paths_map: &BTreeMap<String, PathBuf>) -> BTreeMap<PathBuf, PathBuf> {
    paths_map
        .iter()
        .map(|(key, value)| (PathBuf::from(key), value.to_owned()))
        .collect()
}

/// Inject our default configuration into the provided plugin manifest.
/// This will set `plugin_id` and `host_environment` for use within PDKs.
#[instrument(skip(manifest))]
pub fn inject_default_manifest_config(
    id: &Id,
    home_dir: &Path,
    manifest: &mut Manifest,
) -> miette::Result<()> {
    let os = SystemOS::from_env();
    let env = serde_json::to_string(&HostEnvironment {
        arch: SystemArch::from_env(),
        libc: SystemLibc::detect(os),
        os,
        home_dir: to_virtual_path(
            &map_virtual_paths(manifest.allowed_paths.as_ref().unwrap()),
            home_dir,
        ),
    })
    .into_diagnostic()?;

    trace!(id = id.as_str(), "Storing plugin identifier");

    manifest
        .config
        .insert("plugin_id".to_string(), id.to_string());

    trace!(id = id.as_str(), env = %env, "Storing host environment");

    manifest.config.insert("host_environment".to_string(), env);

    Ok(())
}

/// A container around Extism's [`Plugin`] and [`Manifest`] types that provides convenience
/// methods for calling and caching functions from the WASM plugin. It also provides
/// additional methods for easily working with WASI and virtual paths.
pub struct PluginContainer {
    pub id: Id,
    pub manifest: Manifest,

    func_cache: Arc<scc::HashMap<String, Vec<u8>>>,
    plugin: Arc<RwLock<Plugin>>,
}

unsafe impl Send for PluginContainer {}
unsafe impl Sync for PluginContainer {}

impl PluginContainer {
    /// Create a new container with the provided manifest and host functions.
    #[instrument(name = "new_plugin", skip(manifest, functions))]
    pub fn new(
        id: Id,
        manifest: Manifest,
        functions: impl IntoIterator<Item = Function>,
    ) -> miette::Result<PluginContainer> {
        trace!(id = id.as_str(), "Creating plugin container");

        let plugin = Plugin::new(&manifest, functions, true).map_err(|error| {
            if is_incompatible_runtime(&error) {
                WarpgatePluginError::IncompatibleRuntime { id: id.clone() }
            } else {
                WarpgatePluginError::FailedContainer {
                    id: id.clone(),
                    error: Box::new(error),
                }
            }
        })?;

        trace!(
            id = id.as_str(),
            plugin = plugin.id.to_string(),
            "Created plugin container",
        );

        Ok(PluginContainer {
            manifest,
            plugin: Arc::new(RwLock::new(plugin)),
            id,
            func_cache: Arc::new(scc::HashMap::new()),
        })
    }

    /// Create a new container with the provided manifest.
    pub fn new_without_functions(id: Id, manifest: Manifest) -> miette::Result<PluginContainer> {
        Self::new(id, manifest, [])
    }

    /// Call a function on the plugin with no input and cache the output before returning it.
    /// Subsequent calls will read from the cache.
    pub async fn cache_func<O>(&self, func: &str) -> miette::Result<O>
    where
        O: Debug + DeserializeOwned,
    {
        self.cache_func_with(func, Empty::default()).await
    }

    /// Call a function on the plugin with the given input and cache the output
    /// before returning it. Subsequent calls with the same input will read from the cache.
    #[instrument(skip(self))]
    pub async fn cache_func_with<I, O>(&self, func: &str, input: I) -> miette::Result<O>
    where
        I: Debug + Serialize,
        O: Debug + DeserializeOwned,
    {
        let input = self.format_input(func, input)?;
        let cache_key = format!("{func}-{input}");

        // Check if cache exists already
        if let Some(data) = self.func_cache.get(&cache_key) {
            return self.parse_output(func, data.get());
        }

        // Otherwise call the function and cache the result
        let data = self.call(func, input).await?;
        let output: O = self.parse_output(func, &data)?;

        let _ = self.func_cache.insert(cache_key, data);

        Ok(output)
    }

    /// Call a function on the plugin with no input and return the output.
    pub async fn call_func<O>(&self, func: &str) -> miette::Result<O>
    where
        O: Debug + DeserializeOwned,
    {
        self.call_func_with(func, Empty::default()).await
    }

    /// Call a function on the plugin with the given input and return the output.
    #[instrument(skip(self))]
    pub async fn call_func_with<I, O>(&self, func: &str, input: I) -> miette::Result<O>
    where
        I: Debug + Serialize,
        O: Debug + DeserializeOwned,
    {
        self.parse_output(
            func,
            &self.call(func, self.format_input(func, input)?).await?,
        )
    }

    /// Call a function on the plugin with the given input and ignore the output.
    #[instrument(skip(self))]
    pub async fn call_func_without_output<I>(&self, func: &str, input: I) -> miette::Result<()>
    where
        I: Debug + Serialize,
    {
        self.call(func, self.format_input(func, input)?).await?;
        Ok(())
    }

    /// Return true if the plugin has a function with the given id.
    pub async fn has_func(&self, func: &str) -> bool {
        self.plugin.read().await.function_exists(func)
    }

    /// Convert the provided virtual guest path to an absolute host path.
    pub fn from_virtual_path(&self, path: impl AsRef<Path> + Debug) -> PathBuf {
        let Some(virtual_paths) = self.manifest.allowed_paths.as_ref() else {
            return path.as_ref().to_path_buf();
        };

        let paths = map_virtual_paths(virtual_paths);

        from_virtual_path(&paths, path)
    }

    /// Convert the provided absolute host path to a virtual guest path suitable
    /// for WASI sandboxed runtimes.
    pub fn to_virtual_path(&self, path: impl AsRef<Path> + Debug) -> VirtualPath {
        let Some(virtual_paths) = self.manifest.allowed_paths.as_ref() else {
            return VirtualPath::OnlyReal(path.as_ref().to_path_buf());
        };

        let paths = map_virtual_paths(virtual_paths);

        to_virtual_path(&paths, path)
    }

    /// Call a function on the plugin with the given raw input and return the raw output.
    pub async fn call(&self, func: &str, input: impl AsRef<[u8]>) -> miette::Result<Vec<u8>> {
        let mut instance = self.plugin.write().await;
        let input = input.as_ref();
        let uuid = instance.id.to_string(); // Copy
        let instant = Instant::now();

        trace!(
            id = self.id.as_str(),
            plugin = &uuid,
            input = %String::from_utf8_lossy(input),
            "Calling guest function {}",
            color::property(func),
        );

        let output = block_in_place(|| instance.call(func, input)).map_err(|error| {
            if is_incompatible_runtime(&error) {
                return WarpgatePluginError::IncompatibleRuntime {
                    id: self.id.clone(),
                };
            }

            let message = apply_style_tags(
                error
                    .source()
                    .map(|src| src.to_string())
                    .unwrap_or_else(|| error.to_string())
                    .replace("\\\\n", "\n")
                    .replace("\\n", "\n")
                    .trim(),
            );

            // When in debug mode, include more information around errors.
            #[cfg(debug_assertions)]
            {
                WarpgatePluginError::FailedPluginCall {
                    id: self.id.clone(),
                    func: func.to_owned(),
                    error: message,
                }
            }

            // When in release mode, errors don't render properly with the
            // previous variant, so this is a special variant that renders as-is.
            #[cfg(not(debug_assertions))]
            {
                WarpgatePluginError::FailedPluginCallRelease { error: message }
            }
        })?;

        trace!(
            id = self.id.as_str(),
            plugin = &uuid,
            output = %String::from_utf8_lossy(output),
            elapsed = ?instant.elapsed(),
            "Called guest function {}",
            color::property(func),
        );

        Ok(output.to_vec())
    }

    fn format_input<I: Serialize>(&self, func: &str, input: I) -> miette::Result<String> {
        Ok(
            serde_json::to_string(&input).map_err(|error| WarpgatePluginError::InvalidInput {
                id: self.id.clone(),
                func: func.to_owned(),
                error: Box::new(error),
            })?,
        )
    }

    fn parse_output<O: DeserializeOwned>(&self, func: &str, data: &[u8]) -> miette::Result<O> {
        Ok(
            serde_json::from_slice(data).map_err(|error| WarpgatePluginError::InvalidOutput {
                id: self.id.clone(),
                func: func.to_owned(),
                error: Box::new(error),
            })?,
        )
    }
}
