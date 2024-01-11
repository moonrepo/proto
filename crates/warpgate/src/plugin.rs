use crate::endpoints::Empty;
use crate::error::WarpgateError;
use crate::helpers::{from_virtual_path, to_virtual_path};
use crate::id::Id;
use extism::{Error, Function, Manifest, Plugin};
use once_map::OnceMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use starbase_styles::color;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::trace;
use warpgate_api::VirtualPath;

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

/// A container around Extism's [`Plugin`] and [`Manifest`] types that provides convenience
/// methods for calling and caching functions from the WASM plugin. It also provides
/// additional methods for easily working with WASI and virtual paths.
pub struct PluginContainer {
    pub id: Id,
    pub manifest: Manifest,

    func_cache: OnceMap<String, Vec<u8>>,
    plugin: Arc<RwLock<Plugin>>,
}

unsafe impl Send for PluginContainer {}
unsafe impl Sync for PluginContainer {}

impl PluginContainer {
    /// Create a new container with the provided manifest and host functions.
    pub fn new(
        id: Id,
        manifest: Manifest,
        functions: impl IntoIterator<Item = Function>,
    ) -> miette::Result<PluginContainer> {
        let plugin = Plugin::new(&manifest, functions, true).map_err(|error| {
            if is_incompatible_runtime(&error) {
                WarpgateError::IncompatibleRuntime { id: id.clone() }
            } else {
                WarpgateError::PluginCreateFailed {
                    id: id.clone(),
                    error,
                }
            }
        })?;

        Ok(PluginContainer {
            manifest,
            plugin: Arc::new(RwLock::new(plugin)),
            id,
            func_cache: OnceMap::new(),
        })
    }

    /// Create a new container with the provided manifest.
    pub fn new_without_functions(id: Id, manifest: Manifest) -> miette::Result<PluginContainer> {
        Self::new(id, manifest, [])
    }

    /// Call a function on the plugin with no input and cache the output before returning it.
    /// Subsequent calls will read from the cache.
    pub fn cache_func<O>(&self, func: &str) -> miette::Result<O>
    where
        O: Debug + DeserializeOwned,
    {
        self.cache_func_with(func, Empty::default())
    }

    /// Call a function on the plugin with the given input and cache the output
    /// before returning it. Subsequent calls with the same input will read from the cache.
    pub fn cache_func_with<I, O>(&self, func: &str, input: I) -> miette::Result<O>
    where
        I: Debug + Serialize,
        O: Debug + DeserializeOwned,
    {
        let input = self.format_input(func, input)?;
        let cache_key = format!("{func}-{input}");

        // Check if cache exists already in read-only mode
        {
            if let Some(data) = self.func_cache.get(&cache_key) {
                return self.parse_output(func, data);
            }
        }

        // Otherwise call the function and cache the result
        let data = self.call(func, input)?;
        let output: O = self.parse_output(func, &data)?;

        self.func_cache.insert(cache_key, |_| data);

        Ok(output)
    }

    /// Call a function on the plugin with no input and return the output.
    pub fn call_func<O>(&self, func: &str) -> miette::Result<O>
    where
        O: Debug + DeserializeOwned,
    {
        self.call_func_with(func, Empty::default())
    }

    /// Call a function on the plugin with the given input and return the output.
    pub fn call_func_with<I, O>(&self, func: &str, input: I) -> miette::Result<O>
    where
        I: Debug + Serialize,
        O: Debug + DeserializeOwned,
    {
        self.parse_output(func, &self.call(func, self.format_input(func, input)?)?)
    }

    /// Call a function on the plugin with the given input and ignore the output.
    pub fn call_func_without_output<I>(&self, func: &str, input: I) -> miette::Result<()>
    where
        I: Debug + Serialize,
    {
        self.call(func, self.format_input(func, input)?)?;
        Ok(())
    }

    /// Return true if the plugin has a function with the given id.
    pub fn has_func(&self, func: &str) -> bool {
        self.plugin
            .write()
            .unwrap_or_else(|_| {
                panic!(
                    "Unable to acquire write access to `{}` WASM plugin.",
                    self.id
                )
            })
            .function_exists(func)
    }

    /// Convert the provided virtual guest path to an absolute host path.
    pub fn from_virtual_path(&self, path: &Path) -> PathBuf {
        let Some(virtual_paths) = self.manifest.allowed_paths.as_ref() else {
            return path.to_path_buf();
        };

        from_virtual_path(virtual_paths, path)
    }

    /// Convert the provided absolute host path to a virtual guest path suitable
    /// for WASI sandboxed runtimes.
    pub fn to_virtual_path(&self, path: &Path) -> VirtualPath {
        let Some(virtual_paths) = self.manifest.allowed_paths.as_ref() else {
            return VirtualPath::Only(path.to_path_buf());
        };

        to_virtual_path(virtual_paths, path)
    }

    /// Call a function on the plugin with the given raw input and return the raw output.
    pub fn call(&self, func: &str, input: impl AsRef<[u8]>) -> miette::Result<Vec<u8>> {
        let input = input.as_ref();

        trace!(
            plugin = self.id.as_str(),
            input = %String::from_utf8_lossy(input),
            "Calling plugin function {}",
            color::label(func),
        );

        let mut instance = self.plugin.write().unwrap_or_else(|_| {
            panic!(
                "Unable to acquire write access to `{}` WASM plugin.",
                self.id
            )
        });

        let output = instance.call(func, input).map_err(|error| {
            if is_incompatible_runtime(&error) {
                return WarpgateError::IncompatibleRuntime {
                    id: self.id.clone(),
                };
            }

            let message = error
                .source()
                .map(|src| src.to_string())
                .unwrap_or_else(|| error.to_string());

            // When in debug mode, include more information around errors.
            #[cfg(debug_assertions)]
            {
                WarpgateError::PluginCallFailed {
                    id: self.id.clone(),
                    func: func.to_owned(),
                    error: message,
                }
            }
            // When in release mode, errors don't render properly with the
            // previous variant, so this is a special variant that renders as-is.
            #[cfg(not(debug_assertions))]
            {
                WarpgateError::PluginCallFailedRelease {
                    error: message.replace("\\\\n", "\n").replace("\\n", "\n"),
                }
            }
        })?;

        trace!(
            plugin = self.id.as_str(),
            output = %String::from_utf8_lossy(output),
            "Called plugin function {}",
            color::label(func),
        );

        Ok(output.to_vec())
    }

    fn format_input<I: Serialize>(&self, func: &str, input: I) -> miette::Result<String> {
        Ok(
            serde_json::to_string(&input).map_err(|error| WarpgateError::FormatInputFailed {
                id: self.id.clone(),
                func: func.to_owned(),
                error,
            })?,
        )
    }

    fn parse_output<O: DeserializeOwned>(&self, func: &str, data: &[u8]) -> miette::Result<O> {
        Ok(
            serde_json::from_slice(data).map_err(|error| WarpgateError::ParseOutputFailed {
                id: self.id.clone(),
                func: func.to_owned(),
                error,
            })?,
        )
    }
}
