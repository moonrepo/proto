use crate::api::Empty;
use crate::error::WarpgateError;
use extism::{Function, Manifest, Plugin};
use once_map::OnceMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::trace;

pub struct PluginContainer<'plugin> {
    pub id: String,
    pub manifest: Manifest,

    func_cache: OnceMap<String, Vec<u8>>,
    plugin: Arc<RwLock<Plugin<'plugin>>>,
}

unsafe impl<'plugin> Send for PluginContainer<'plugin> {}
unsafe impl<'plugin> Sync for PluginContainer<'plugin> {}

impl<'plugin> PluginContainer<'plugin> {
    /// Create a new container with the provided manifest and host functions.
    pub fn new<'new>(
        id: &str,
        manifest: Manifest,
        functions: impl IntoIterator<Item = Function>,
    ) -> miette::Result<PluginContainer<'new>> {
        let plugin = Plugin::create_with_manifest(&manifest, functions, true)
            .map_err(|error| WarpgateError::PluginCreateFailed { error })?;

        Ok(PluginContainer {
            manifest,
            plugin: Arc::new(RwLock::new(plugin)),
            id: id.to_owned(),
            func_cache: OnceMap::new(),
        })
    }

    /// Create a new container with the provided manifest.
    pub fn new_without_functions<'new>(
        id: &str,
        manifest: Manifest,
    ) -> miette::Result<PluginContainer<'new>> {
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
        let output: O = self.parse_output(func, data)?;

        self.func_cache.insert(cache_key, |_| data.to_vec());

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
        self.parse_output(func, self.call(func, self.format_input(func, input)?)?)
    }

    /// Return true if the plugin has a function with the given id.
    pub fn has_func(&self, func: &str) -> bool {
        self.plugin
            .read()
            .unwrap_or_else(|_| {
                panic!(
                    "Unable to acquire read access to `{}` WASM plugin.",
                    self.id
                )
            })
            .has_function(func)
    }

    /// Convert the provided absolute host path to a virtual guest path suitable
    /// for WASI sandboxed runtimes.
    pub fn to_virtual_path(&self, path: &Path) -> PathBuf {
        let Some(virtual_paths) = self.manifest.allowed_paths.as_ref() else {
            return path.to_path_buf();
        };

        for (host_path, guest_path) in virtual_paths {
            if path.starts_with(host_path) {
                let path = guest_path.join(path.strip_prefix(host_path).unwrap());

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

    fn call(&self, func: &str, input: impl AsRef<[u8]>) -> miette::Result<&[u8]> {
        let input = input.as_ref();

        let output = self
            .plugin
            .write()
            .unwrap_or_else(|_| {
                panic!(
                    "Unable to acquire write access to `{}` WASM plugin.",
                    self.id
                )
            })
            .call(func, input)
            .map_err(|error| WarpgateError::PluginCallFailed {
                func: func.to_owned(),
                error,
            })?;

        trace!(
            plugin = self.id,
            func,
            input = %String::from_utf8_lossy(input),
            output = %String::from_utf8_lossy(output),
            "Called plugin function"
        );

        Ok(output)
    }

    fn format_input<I: Serialize>(&self, func: &str, input: I) -> miette::Result<String> {
        Ok(
            serde_json::to_string(&input).map_err(|error| WarpgateError::FormatInputFailed {
                func: func.to_owned(),
                error,
            })?,
        )
    }

    fn parse_output<O: DeserializeOwned>(&self, func: &str, data: &[u8]) -> miette::Result<O> {
        Ok(
            serde_json::from_slice(data).map_err(|error| WarpgateError::ParseOutputFailed {
                func: func.to_owned(),
                error,
            })?,
        )
    }
}
