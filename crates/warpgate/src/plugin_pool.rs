use crate::plugin::map_container_error;
use crate::plugin_error::WarpgatePluginError;
use extism::{CompiledPlugin, Manifest, Plugin};
use std::env;
use std::sync::{Arc, Mutex};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use warpgate_api::Id;

/// The manifest configuration key that plugin consumers can define to
/// allow multiple instances of the plugin to execute calls in parallel.
pub const MAX_INSTANCES_CONFIG_KEY: &str = "plugin_instances";

fn determine_max_instances(manifest: &Manifest) -> usize {
    // Environment variable takes precedence as a global override,
    // primarily for debugging and testing purposes
    if let Ok(value) = env::var("WARPGATE_PLUGIN_INSTANCES")
        && let Ok(count) = value.parse::<usize>()
        && count > 0
    {
        return count;
    }

    // Otherwise the host application must opt-in each plugin,
    // as it requires the plugin to not rely on cross-call state
    if let Some(value) = manifest.config.get(MAX_INSTANCES_CONFIG_KEY)
        && let Ok(count) = value.parse::<usize>()
        && count > 0
    {
        return count.min(16);
    }

    // Extism variables (and other guest state) are stored per instance,
    // and many plugins use them as a cross-call cache, so a single
    // instance must be the default to preserve existing semantics!
    1
}

/// A bounded pool of instances created from a single pre-compiled plugin.
/// Each guest call checks out an instance exclusively, so concurrent calls
/// execute in parallel across instances instead of serializing on one.
///
/// The pool defaults to a single instance, which matches the historical
/// behavior of one long-lived instance per plugin. Consumers can allow
/// parallel execution per plugin with the `plugin_instances` manifest
/// configuration key, but only for plugins that do not rely on in-memory
/// guest state (like Extism variables) persisting across separate calls,
/// as each call may then execute on a different instance.
pub struct PluginInstancePool {
    compiled: CompiledPlugin,
    idle: Mutex<Vec<Plugin>>,
    limiter: Arc<Semaphore>,
}

impl PluginInstancePool {
    /// Create a new pool from the pre-compiled plugin, seeded with the
    /// provided instances. The maximum number of live instances is defined
    /// by the manifest configuration, or the `WARPGATE_PLUGIN_INSTANCES`
    /// environment variable, and defaults to 1.
    pub fn new(compiled: CompiledPlugin, manifest: &Manifest, instances: Vec<Plugin>) -> Self {
        Self {
            compiled,
            idle: Mutex::new(instances),
            limiter: Arc::new(Semaphore::new(determine_max_instances(manifest))),
        }
    }

    /// Check out an idle instance, or create a new one from the pre-compiled
    /// plugin (cheap, as no compilation occurs). Waits when the maximum
    /// number of instances are all busy.
    pub async fn acquire(
        &self,
        id: &Id,
    ) -> Result<(Plugin, OwnedSemaphorePermit), WarpgatePluginError> {
        let permit = Arc::clone(&self.limiter)
            .acquire_owned()
            .await
            .expect("Plugin instance limiter has been closed!");

        let instance = self.idle.lock().unwrap().pop();

        let instance = match instance {
            Some(instance) => instance,
            None => Plugin::new_from_compiled(&self.compiled)
                .map_err(|error| map_container_error(id, error))?,
        };

        Ok((instance, permit))
    }

    /// Return an instance to the pool once its call has finished. Instances
    /// that trapped during their call are discarded instead of reused, since
    /// the guest's internal state can no longer be trusted (memory may be
    /// corrupted), and re-creating an instance is cheap. Clean guest errors
    /// keep the instance, as plugins return errors during normal operation,
    /// and their state (like Extism variables) must persist across calls.
    pub fn restore(&self, instance: Plugin, permit: OwnedSemaphorePermit, reusable: bool) {
        if reusable {
            self.idle.lock().unwrap().push(instance);
        }

        drop(permit);
    }
}
