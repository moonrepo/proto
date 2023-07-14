use crate::api::Empty;
use crate::error::WarpgateError;
use extism::Plugin;
use once_map::OnceMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use tracing::trace;

pub struct PluginContainer<'plugin> {
    pub plugin: Arc<RwLock<Plugin<'plugin>>>,
    pub name: String,

    func_cache: OnceMap<String, Vec<u8>>,
}

impl<'plugin> PluginContainer<'plugin> {
    /// Create a new container with the provided plugin.
    pub fn new<'new>(name: &str, plugin: Plugin<'new>) -> PluginContainer<'new> {
        PluginContainer {
            plugin: Arc::new(RwLock::new(plugin)),
            name: name.to_owned(),
            func_cache: OnceMap::new(),
        }
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

    /// Return true if the plugin has a function with the given name.
    pub fn has_func(&self, func: &str) -> bool {
        self.plugin
            .read()
            .expect(
                format!(
                    "Unable to acquire read access to `{}` WASM plugin.",
                    self.name
                )
                .as_str(),
            )
            .has_function(func)
    }

    fn call(&self, func: &str, input: impl AsRef<[u8]>) -> miette::Result<&[u8]> {
        let input = input.as_ref();

        let output = self
            .plugin
            .write()
            .expect(
                format!(
                    "Unable to acquire write access to `{}` WASM plugin.",
                    self.name
                )
                .as_str(),
            )
            .call(func, input)
            .map_err(|error| WarpgateError::PluginCallFailed {
                func: func.to_owned(),
                error,
            })?;

        trace!(
            plugin = self.name,
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
