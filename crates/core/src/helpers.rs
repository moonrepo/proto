use ai_env::AiNetworkPolicy;
use regex::Regex;
use serde::Serialize;
use serde::de::DeserializeOwned;
use starbase_archive::is_supported_archive_extension;
use starbase_utils::{
    envx::{self, bool_var},
    fs,
    json::{self, JsonError},
    net,
};
use std::env;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, OnceLock};
use std::time::SystemTime;
use version_spec::Version;
use warpgate::RegistryConfig;

pub static ENV_VAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$(?<name>[A-Z0-9_]+)").unwrap());

pub static ENV_VAR_SUB: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{(?<name>[A-Z0-9_]+)\}").unwrap());

pub fn get_proto_version() -> &'static Version {
    static VERSION_CACHE: OnceLock<Version> = OnceLock::new();

    VERSION_CACHE.get_or_init(|| {
        Version::parse(
            env::var("PROTO_VERSION")
                .ok()
                .as_deref()
                .unwrap_or(env!("CARGO_PKG_VERSION")),
        )
        .unwrap()
    })
}

pub fn get_builtin_registry() -> &'static RegistryConfig {
    static PROTO_BUILTIN_REGISTRY: OnceLock<RegistryConfig> = OnceLock::new();

    PROTO_BUILTIN_REGISTRY.get_or_init(|| {
        let registry = env::var("PROTO_BUILTIN_REGISTRY_HOST")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "ghcr.io".to_string());

        let namespace = env::var("PROTO_BUILTIN_REGISTRY_NAMESPACE")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "moonrepo".to_string());

        let auth = bool_var("PROTO_BUILTIN_REGISTRY_AUTH");

        let default = bool_var("PROTO_BUILTIN_REGISTRY_DEFAULT");

        RegistryConfig {
            auth,
            default,
            registry,
            namespace: Some(namespace),
        }
    })
}

pub fn is_offline() -> bool {
    static OFFLINE_CACHE: OnceLock<bool> = OnceLock::new();

    *OFFLINE_CACHE.get_or_init(|| {
        if let Ok(value) = env::var("PROTO_OFFLINE") {
            match value.as_ref() {
                "1" | "true" => return true,
                "0" | "false" => return false,
                _ => {}
            };
        }

        if let Some(agent_env) = ai_env::get_environment() {
            match agent_env.network {
                AiNetworkPolicy::Disabled => return true,
                AiNetworkPolicy::Open | AiNetworkPolicy::Filtered => return false,
                _ => {}
            };
        }

        let override_default = envx::bool_var("PROTO_OFFLINE_OVERRIDE_HOSTS");

        let timeout: u64 = env::var("PROTO_OFFLINE_TIMEOUT")
            .map(|value| value.parse().expect("Invalid offline timeout."))
            .unwrap_or(750);

        let custom_hosts: Vec<String> = env::var("PROTO_OFFLINE_HOSTS")
            .map(|value| value.split(',').map(|v| v.trim().to_owned()).collect())
            .unwrap_or_default();

        let ip_version = env::var("PROTO_OFFLINE_IP_VERSION").unwrap_or_default();

        net::is_offline_with_options(net::OfflineOptions {
            check_default_hosts: !override_default,
            check_default_ips: !override_default,
            custom_hosts,
            custom_ips: vec![],
            ip_v4: ip_version.is_empty() || ip_version == "4",
            ip_v6: ip_version.is_empty() || ip_version == "6",
            timeout,
        })
    })
}

pub fn is_cache_enabled() -> bool {
    match env::var("PROTO_CACHE") {
        Ok(value) => value != "0" && value != "false" && value != "no" && value != "off",
        Err(_) => true,
    }
}

pub fn is_archive_file<P: AsRef<Path>>(path: P) -> bool {
    is_supported_archive_extension(path.as_ref())
}

pub fn now() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub fn read_json_file_with_lock<T: DeserializeOwned>(
    path: impl AsRef<Path>,
) -> Result<T, JsonError> {
    let path = path.as_ref();
    let mut content = fs::read_file_with_lock(path)?;

    // When multiple processes are ran in parallel, we may run into an issue where
    // the file has been truncated, so JSON parsing fails. It's a rare race condition,
    // and these file locks don't seem to catch it. If this happens, fallback to empty JSON.
    // https://github.com/moonrepo/proto/issues/85
    if content.is_empty() {
        content = "{}".into();
    }

    let data: T = json::serde_json::from_str(&content).map_err(|error| JsonError::ReadFile {
        path: path.to_path_buf(),
        error: Box::new(error),
    })?;

    Ok(data)
}

pub fn write_json_file_with_lock<T: Serialize>(
    path: impl AsRef<Path>,
    data: &T,
) -> Result<(), JsonError> {
    let path = path.as_ref();

    let data = json::serde_json::to_string_pretty(data).map_err(|error| JsonError::WriteFile {
        path: path.to_path_buf(),
        error: Box::new(error),
    })?;

    fs::write_file_with_lock(path, data)?;

    Ok(())
}

/// Serialize and write JSON to the provided path by writing to a temporary file
/// in the same directory, then atomically renaming it over the destination.
/// Unlike a truncate-then-write, another process can never observe an empty or
/// partially written file, even if this process is killed mid-write.
/// https://github.com/moonrepo/proto/issues/1057
pub fn write_json_file_atomic<T: Serialize>(
    path: impl AsRef<Path>,
    data: &T,
) -> Result<(), JsonError> {
    static TEMP_COUNT: AtomicU64 = AtomicU64::new(0);

    let path = path.as_ref();

    let data = json::serde_json::to_string_pretty(data).map_err(|error| JsonError::WriteFile {
        path: path.to_path_buf(),
        error: Box::new(error),
    })?;

    let temp_path = path.with_extension(format!(
        "{}-{}.tmp",
        std::process::id(),
        TEMP_COUNT.fetch_add(1, Ordering::Relaxed)
    ));

    fs::write_file(&temp_path, data)?;

    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);

        return Err(error.into());
    }

    Ok(())
}

/// Cloning an entire map, like `IndexMap`, is very costly as it clones the entire structure.
/// This helper allows you to clone just the keys and values, which is much faster if you
/// don't need the map features.
pub fn fast_map_clone<'map, I, K, V>(items: I) -> Vec<(K, V)>
where
    I: IntoIterator<Item = (&'map K, &'map V)>,
    K: Clone + 'map,
    V: Clone + 'map,
{
    items
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

/// Cloning an entire list, like `Vec`, is very costly as it clones the entire structure.
/// This helper allows you to clone just the values, which is much faster if you don't
/// need the list features.
pub fn fast_list_clone<'map, I, V>(items: I) -> Vec<V>
where
    I: IntoIterator<Item = &'map V>,
    V: Clone + 'map,
{
    items.into_iter().map(|v| v.to_owned()).collect()
}
