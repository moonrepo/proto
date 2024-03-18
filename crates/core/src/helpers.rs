use crate::error::ProtoError;
use cached::proc_macro::cached;
use miette::IntoDiagnostic;
use once_cell::sync::Lazy;
use regex::Regex;
use semver::Version;
use serde::de::DeserializeOwned;
use serde::Serialize;
use starbase_archive::is_supported_archive_extension;
use starbase_utils::dirs::home_dir;
use starbase_utils::fs;
use starbase_utils::json::{self, JsonError};
use starbase_utils::net;
use std::path::Path;
use std::time::SystemTime;
use std::{env, path::PathBuf};

pub static ENV_VAR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$(?<name>[A-Z0-9_]+)").unwrap());

pub static ENV_VAR_SUB: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\$\{(?<name>[A-Z0-9_]+)\}").unwrap());

#[cached]
pub fn get_proto_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).unwrap()
}

pub fn get_proto_home() -> miette::Result<PathBuf> {
    if let Ok(root) = env::var("PROTO_HOME") {
        return Ok(root.into());
    }

    if let Ok(root) = env::var("PROTO_ROOT") {
        return Ok(root.into());
    }

    Ok(get_home_dir()?.join(".proto"))
}

pub fn get_home_dir() -> miette::Result<PathBuf> {
    Ok(home_dir().ok_or(ProtoError::MissingHomeDir)?)
}

#[cached(time = 300)]
#[tracing::instrument]
pub fn is_offline() -> bool {
    if let Ok(value) = env::var("PROTO_OFFLINE") {
        match value.as_ref() {
            "1" | "true" => return true,
            "0" | "false" => return false,
            _ => {}
        };
    }

    let timeout: u64 = env::var("PROTO_OFFLINE_TIMEOUT")
        .map(|v| v.parse().expect("Invalid offline timeout."))
        .unwrap_or(750);

    let hosts = env::var("PROTO_OFFLINE_HOSTS")
        .map(|value| {
            value
                .split(',')
                .map(|v| v.trim().to_owned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    net::is_offline(timeout, hosts)
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
        error,
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
        error,
    })?;

    fs::write_file_with_lock(path, data)?;

    Ok(())
}
