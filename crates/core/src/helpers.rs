use regex::Regex;
use semver::Version;
use serde::Serialize;
use serde::de::DeserializeOwned;
use starbase_archive::is_supported_archive_extension;
use starbase_utils::{
    envx, fs,
    json::{self, JsonError},
    net,
};
use std::env;
use std::path::Path;
use std::sync::{LazyLock, OnceLock};
use std::time::SystemTime;

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

#[cfg(unix)]
pub fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    use std::os::unix::fs::PermissionsExt;

    fs::metadata(path.as_ref())
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(windows)]
pub fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().extension().is_some_and(|ext| ext == "exe")
}

pub fn now() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub fn extract_filename_from_url<U: AsRef<str>>(url: U) -> String {
    let base = url.as_ref();

    match url::Url::parse(base) {
        Ok(url) => {
            let mut segments = url.path_segments().unwrap();

            segments.next_back().unwrap().to_owned()
        }
        Err(_) => if let Some(i) = base.rfind('/') {
            &base[i + 1..]
        } else {
            "unknown"
        }
        .into(),
    }
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
