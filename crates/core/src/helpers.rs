use miette::IntoDiagnostic;
use regex::Regex;
use semver::Version;
use serde::de::DeserializeOwned;
use serde::Serialize;
use starbase_archive::is_supported_archive_extension;
use starbase_utils::env::bool_var;
use starbase_utils::fs;
use starbase_utils::json::{self, JsonError};
use starbase_utils::net;
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

    VERSION_CACHE.get_or_init(|| Version::parse(env!("CARGO_PKG_VERSION")).unwrap())
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

        let override_default = bool_var("PROTO_OFFLINE_OVERRIDE_HOSTS");

        let timeout: u64 = env::var("PROTO_OFFLINE_TIMEOUT")
            .map(|value| value.parse().expect("Invalid offline timeout."))
            .unwrap_or(750);

        let custom_hosts: Vec<String> = env::var("PROTO_OFFLINE_HOSTS")
            .map(|value| value.split(',').map(|v| v.trim().to_owned()).collect())
            .unwrap_or_default();

        net::is_offline_with_options(net::OfflineOptions {
            check_default_hosts: !override_default,
            check_default_ips: !override_default,
            custom_hosts,
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

pub fn extract_filename_from_url<U: AsRef<str>>(url: U) -> miette::Result<String> {
    let url = url::Url::parse(url.as_ref()).into_diagnostic()?;
    let segments = url.path_segments().unwrap();

    Ok(segments.last().unwrap().to_owned())
}

pub fn read_json_file_with_lock<T: DeserializeOwned>(path: impl AsRef<Path>) -> miette::Result<T> {
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
) -> miette::Result<()> {
    let path = path.as_ref();

    let data = json::serde_json::to_string_pretty(data).map_err(|error| JsonError::WriteFile {
        path: path.to_path_buf(),
        error: Box::new(error),
    })?;

    fs::write_file_with_lock(path, data)?;

    Ok(())
}
