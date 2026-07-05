use crate::helpers::{from_virtual_path, sort_virtual_paths, to_virtual_path};
use crate::plugin_error::WarpgatePluginError;
use crate::plugin_pool::PluginInstancePool;
use extism::{Error, Function, Manifest, Plugin, PluginBuilder};
use scc::hash_map::Entry;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use starbase_styles::{apply_style_tags, color};
use starbase_utils::{
    envx::{bool_var, is_ci},
    hash,
};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use system_env::{SystemArch, SystemLibc, SystemOS};
use tokio::sync::OnceCell;
use tokio::task::spawn_blocking;
use tracing::{instrument, trace};
use warpgate_api::{HostEnvironment, Id, VirtualPath};

pub(crate) fn is_incompatible_runtime(error: &Error) -> bool {
    let check = |message: String| {
        // unknown import: `env::exec_command` has not been defined
        message.contains("unknown import") && message.contains("env::")
    };

    if let Some(source) = error.source()
        && check(source.to_string())
    {
        return true;
    }

    check(error.to_string())
}

pub(crate) fn map_container_error(id: &Id, error: Error) -> WarpgatePluginError {
    if is_incompatible_runtime(&error) {
        WarpgatePluginError::IncompatibleRuntime { id: id.to_owned() }
    } else {
        WarpgatePluginError::FailedContainer {
            id: id.to_owned(),
            error: Box::new(error),
        }
    }
}

// The trap types are not exposed through Extism's public API,
// so we must detect them based on known error messages.
pub(crate) fn is_trap_error(error: &Error) -> bool {
    error.chain().any(|cause| {
        let message = cause.to_string();

        message.contains("wasm trap")
            || message.contains("out of fuel")
            || message == "timeout"
            || message == "oom"
    })
}

/// Inject our default configuration into the provided plugin manifest.
/// This will set `plugin_id` and `host_environment` for use within PDKs.
#[instrument(skip(manifest))]
pub fn inject_default_manifest_config(
    id: &Id,
    home_dir: &Path,
    manifest: &mut Manifest,
) -> Result<(), WarpgatePluginError> {
    if !manifest.config.contains_key("plugin_id") {
        trace!(id = id.as_str(), "Storing plugin identifier");

        manifest.config.insert("plugin_id".into(), id.to_string());
    }

    if !manifest.config.contains_key("host_environment") {
        let os = SystemOS::from_env();

        let env = serde_json::to_string(&HostEnvironment {
            arch: SystemArch::from_env(),
            ci: is_ci(),
            libc: SystemLibc::detect(os),
            os,
            home_dir: VirtualPath::Virtual {
                path: "/userhome".into(),
                virtual_prefix: "/userhome".into(),
                real_prefix: home_dir.into(),
            },
        })
        .map_err(|error| WarpgatePluginError::InvalidInput {
            id: id.to_owned(),
            func: "host_environment".into(),
            error: Box::new(error),
        })?;

        trace!(id = id.as_str(), env = %env, "Storing host environment");

        manifest.config.insert("host_environment".into(), env);
    }

    Ok(())
}

pub type OnCallFn = Arc<dyn Fn(&str, Option<&str>, Option<&str>) + Send + Sync>;

/// A container around Extism's [`Plugin`] and [`Manifest`] types that provides convenience
/// methods for calling and caching functions from the WASM plugin. It also provides
/// additional methods for easily working with WASI and virtual paths.
///
/// The WASM file is compiled once, while plugin instances are created on demand
/// and pooled, defaulting to a single instance, which matches the historical
/// behavior. Consumers can allow parallel execution per plugin with the
/// `plugin_instances` manifest configuration key, but only for plugins that do
/// not rely on in-memory guest state (like Extism variables) persisting across
/// separate calls, as each call may then execute on a different instance.
pub struct PluginContainer {
    pub id: Id,
    pub manifest: Manifest,
    pub virtual_paths: Vec<(PathBuf, PathBuf)>,

    debug_call: bool,
    func_cache: Arc<scc::HashMap<String, Arc<OnceCell<Vec<u8>>>>>,
    on_call_func: Arc<OnceLock<OnCallFn>>,
    instances: Arc<PluginInstancePool>,
}

impl PluginContainer {
    /// Create a new container with the provided manifest and host functions.
    #[instrument(name = "new_plugin", skip(manifest, functions))]
    pub fn new(
        id: Id,
        manifest: Manifest,
        functions: impl IntoIterator<Item = Function>,
    ) -> Result<PluginContainer, WarpgatePluginError> {
        trace!(id = id.as_str(), "Creating plugin container");

        let compiled = PluginBuilder::new(&manifest)
            .with_functions(functions)
            .with_wasi(true)
            .compile()
            .map_err(|error| map_container_error(&id, error))?;

        // Create the first instance eagerly so that runtime incompatibilities,
        // like missing host functions, error at registration instead of on
        // the first call.
        let instance = Plugin::new_from_compiled(&compiled)
            .map_err(|error| map_container_error(&id, error))?;

        trace!(
            id = id.as_str(),
            plugin = instance.id.to_string(),
            "Created plugin container",
        );

        let mut virtual_paths = match manifest.allowed_paths.as_ref() {
            Some(paths) => paths
                .iter()
                .map(|(host, guest)| (PathBuf::from(host), guest.to_owned()))
                .collect(),
            None => Vec::new(),
        };

        sort_virtual_paths(&mut virtual_paths);

        let instances = Arc::new(PluginInstancePool::new(compiled, &manifest, vec![instance]));

        Ok(PluginContainer {
            virtual_paths,
            manifest,
            instances,
            id,
            func_cache: Arc::new(scc::HashMap::new()),
            on_call_func: Arc::new(OnceLock::new()),
            debug_call: bool_var("WARPGATE_DEBUG_CALL"),
        })
    }

    /// Create a new container with the provided manifest.
    pub fn new_without_functions(
        id: Id,
        manifest: Manifest,
    ) -> Result<PluginContainer, WarpgatePluginError> {
        Self::new(id, manifest, [])
    }

    /// Set a callback handler to be executed when calling a plugin function.
    pub fn set_on_call(&self, func: OnCallFn) {
        let _ = self.on_call_func.set(func);
    }

    /// Call a function on the plugin with no input and cache the output before returning it.
    /// Subsequent calls will read from the cache.
    pub async fn cache_func<F, O>(&self, func: F) -> Result<O, WarpgatePluginError>
    where
        F: Debug + AsRef<str>,
        O: Debug + DeserializeOwned,
    {
        self.cache_func_with(func, Empty::default()).await
    }

    /// Call a function on the plugin with the given input and cache the output
    /// before returning it. Subsequent calls with the same input will read from
    /// the cache, while concurrent calls with the same input will only execute
    /// the function once (single-flight).
    #[instrument(skip(self))]
    pub async fn cache_func_with<F, I, O>(
        &self,
        func: F,
        input: I,
    ) -> Result<O, WarpgatePluginError>
    where
        F: Debug + AsRef<str>,
        I: Debug + Serialize,
        O: Debug + DeserializeOwned,
    {
        let func = func.as_ref();
        let input = self.format_input(func, input)?;
        let cache_key = format!("{func}-{}", hash::base64::from_bytes(&input));

        // Insert the cell synchronously so that the map's shard lock is never
        // held while the function is being called (which may take minutes).
        // The cell itself provides the single-flight semantics.
        let cell = match self.func_cache.entry_async(cache_key).await {
            Entry::Occupied(entry) => Arc::clone(entry.get()),
            Entry::Vacant(entry) => Arc::clone(entry.insert_entry(Arc::new(OnceCell::new())).get()),
        };

        // If the call fails, the cell remains empty, so that
        // subsequent calls can attempt the function again.
        let data = cell.get_or_try_init(|| self.call(func, input)).await?;

        self.parse_output(func, data)
    }

    /// Call a function on the plugin with no input and return the output.
    pub async fn call_func<F, O>(&self, func: F) -> Result<O, WarpgatePluginError>
    where
        F: Debug + AsRef<str>,
        O: Debug + DeserializeOwned,
    {
        self.call_func_with(func, Empty::default()).await
    }

    /// Call a function on the plugin with the given input and return the output.
    #[instrument(skip(self))]
    pub async fn call_func_with<F, I, O>(&self, func: F, input: I) -> Result<O, WarpgatePluginError>
    where
        F: Debug + AsRef<str>,
        I: Debug + Serialize,
        O: Debug + DeserializeOwned,
    {
        let func = func.as_ref();

        self.parse_output(
            func,
            &self.call(func, self.format_input(func, input)?).await?,
        )
    }

    /// Call a function on the plugin with the given input and ignore the output.
    #[instrument(skip(self))]
    pub async fn call_func_without_output<F, I>(
        &self,
        func: F,
        input: I,
    ) -> Result<(), WarpgatePluginError>
    where
        F: Debug + AsRef<str>,
        I: Debug + Serialize,
    {
        let func = func.as_ref();

        self.call(func, self.format_input(func, input)?).await?;

        Ok(())
    }

    /// Return true if the plugin has a function with the given id.
    #[instrument(skip(self))]
    pub async fn has_func(&self, func: impl AsRef<str> + Debug) -> bool {
        let func = func.as_ref();

        let cell = match self.func_cache.entry_async(func.into()).await {
            Entry::Occupied(entry) => Arc::clone(entry.get()),
            Entry::Vacant(entry) => Arc::clone(entry.insert_entry(Arc::new(OnceCell::new())).get()),
        };

        cell.get_or_try_init(|| async {
            let (instance, permit) = self.instances.acquire(&self.id).await?;

            // This only inspects module metadata and does not execute
            // any guest code, so the instance is always reusable.
            let exists = instance.function_exists(func);

            self.instances.restore(instance, permit, true);

            Ok::<_, WarpgatePluginError>(vec![exists as u8])
        })
        .await
        .map(|data| data[0] == 1)
        .unwrap_or(false)
    }

    /// Convert the provided virtual guest path to an absolute host path.
    pub fn from_virtual_path(&self, path: impl AsRef<Path> + Debug) -> PathBuf {
        from_virtual_path(&self.virtual_paths, path)
    }

    /// Convert the provided absolute host path to a virtual guest path suitable
    /// for WASI sandboxed runtimes.
    pub fn to_virtual_path(&self, path: impl AsRef<Path> + Debug) -> VirtualPath {
        to_virtual_path(&self.virtual_paths, path)
    }

    /// Call a function on the plugin with the given raw input and return the raw output.
    #[instrument(skip(self, input))]
    pub async fn call(
        &self,
        func: &str,
        input: impl AsRef<[u8]>,
    ) -> Result<Vec<u8>, WarpgatePluginError> {
        let input = input.as_ref().to_vec();
        let input_string = String::from_utf8_lossy(&input).into_owned();
        let instant = Instant::now();
        let truncate_size = 5000;

        // Check out an instance from the pool, waiting when all
        // instances are busy with other calls
        let (mut instance, permit) = self.instances.acquire(&self.id).await?;
        let uuid = instance.id.to_string(); // Copy

        trace!(
            id = self.id.as_str(),
            plugin = &uuid,
            input = %(if input_string.len() > truncate_size && !self.debug_call {
                "(truncated)"
            } else {
                &input_string
            }),
            "Calling guest function {}",
            color::property(func),
        );

        if let Some(callback) = self.on_call_func.get() {
            callback(func, Some(&input_string), None);
        }

        let func_name = func.to_owned();
        let pool = Arc::clone(&self.instances);

        // Guest calls block the current thread, so execute them on the
        // blocking pool. The instance is restored within the closure so
        // that it is never lost if this future is dropped mid-call.
        let output = spawn_blocking(move || {
            let result = instance
                .call::<&[u8], &[u8]>(&func_name, &input)
                .map(|output| output.to_vec());

            // Keep the instance when the call succeeded or a clean error
            // was returned, and only discard it when the guest trapped
            let reusable = match &result {
                Ok(_) => true,
                Err(error) => !is_trap_error(error),
            };

            pool.restore(instance, permit, reusable);

            result
        })
        .await
        .unwrap_or_else(|error| Err(Error::msg(error.to_string())))
        .map_err(|error| {
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

        let output_string = String::from_utf8_lossy(&output);

        trace!(
            id = self.id.as_str(),
            plugin = &uuid,
            output = %(if output_string.len() > truncate_size && !self.debug_call {
                "(truncated)"
            } else {
                &output_string
            }),
            elapsed = ?instant.elapsed(),
            "Called guest function {}",
            color::property(func),
        );

        if let Some(callback) = self.on_call_func.get() {
            callback(func, None, Some(&output_string));
        }

        Ok(output)
    }

    fn format_input<I: Serialize>(
        &self,
        func: &str,
        input: I,
    ) -> Result<String, WarpgatePluginError> {
        serde_json::to_string(&input).map_err(|error| WarpgatePluginError::InvalidInput {
            id: self.id.clone(),
            func: func.to_owned(),
            error: Box::new(error),
        })
    }

    fn parse_output<O: DeserializeOwned>(
        &self,
        func: &str,
        data: &[u8],
    ) -> Result<O, WarpgatePluginError> {
        serde_json::from_slice(data).map_err(|error| WarpgatePluginError::InvalidOutput {
            id: self.id.clone(),
            func: func.to_owned(),
            error: Box::new(error),
        })
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Empty {}
