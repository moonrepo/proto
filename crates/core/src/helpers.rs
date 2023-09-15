use crate::error::ProtoError;
use cached::proc_macro::cached;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};
use starbase_archive::is_supported_archive_extension;
use starbase_utils::dirs::home_dir;
use starbase_utils::fs::{self, FsError};
use starbase_utils::json::{self, JsonError};
use std::io;
use std::path::Path;
use std::{env, path::PathBuf};
use tracing::trace;

pub static CLEAN_VERSION: Lazy<Regex> = Lazy::new(|| Regex::new(r"([><]=?)\s+(\d)").unwrap());
pub static ENV_VAR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$([A-Z0-9_]+)").unwrap());

#[deprecated = "Use `get_proto_home` instead."]
pub fn get_root() -> miette::Result<PathBuf> {
    get_proto_home()
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

pub fn get_bin_dir() -> miette::Result<PathBuf> {
    Ok(get_proto_home()?.join("bin"))
}

pub fn get_temp_dir() -> miette::Result<PathBuf> {
    Ok(get_proto_home()?.join("temp"))
}

pub fn get_tools_dir() -> miette::Result<PathBuf> {
    Ok(get_proto_home()?.join("tools"))
}

pub fn get_plugins_dir() -> miette::Result<PathBuf> {
    Ok(get_proto_home()?.join("plugins"))
}

// Aliases are words that map to version. For example, "latest" -> "1.2.3".
pub fn is_alias_name<T: AsRef<str>>(value: T) -> bool {
    let value = value.as_ref();

    value.chars().enumerate().all(|(i, c)| {
        if i == 0 {
            char::is_ascii_alphabetic(&c)
        } else {
            char::is_ascii_alphanumeric(&c)
                || c == '-'
                || c == '_'
                || c == '/'
                || c == '.'
                || c == '*'
        }
    })
}

pub fn remove_v_prefix<T: AsRef<str>>(value: T) -> String {
    let value = value.as_ref();

    if value.starts_with('v') || value.starts_with('V') {
        return value[1..].to_owned();
    }

    value.to_owned()
}

pub fn remove_space_after_gtlt<T: AsRef<str>>(value: T) -> String {
    CLEAN_VERSION
        .replace_all(value.as_ref(), "$1$2")
        .to_string()
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

    use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let mut addresses = vec![];

    if let Ok(addrs) = "google.com:80".to_socket_addrs() {
        addresses.extend(addrs);
    }

    addresses.extend([
        // Cloudflare DNS: https://1.1.1.1/dns/
        SocketAddr::from(([1, 1, 1, 1], 53)),
        SocketAddr::from(([1, 0, 0, 1], 53)),
        // Google DNS: https://developers.google.com/speed/public-dns
        SocketAddr::from(([8, 8, 8, 8], 53)),
        SocketAddr::from(([8, 8, 4, 4], 53)),
    ]);

    for address in addresses {
        if TcpStream::connect_timeout(&address, Duration::new(1, 0)).is_ok() {
            return false;
        }
    }

    true
}

pub fn is_cache_enabled() -> bool {
    env::var("PROTO_CACHE").map_or(true, |value| {
        value != "0" && value != "false" && value != "no" && value != "off"
    })
}

#[cached]
#[cfg(windows)]
pub fn is_command_on_path(name: String) -> bool {
    let Ok(system_path) = env::var("PATH") else {
        return false;
    };
    let Ok(path_ext) = env::var("PATHEXT") else {
        return false;
    };
    let exts = path_ext.split(';').collect::<Vec<_>>();

    for path_dir in env::split_paths(&system_path) {
        for ext in &exts {
            if path_dir.join(format!("{name}{ext}")).exists() {
                return true;
            }
        }
    }

    false
}

#[cached]
#[cfg(not(windows))]
pub fn is_command_on_path(name: String) -> bool {
    let Ok(system_path) = env::var("PATH") else {
        return false;
    };

    for path_dir in env::split_paths(&system_path) {
        #[allow(clippy::needless_borrow)]
        if path_dir.join(&name).exists() {
            return true;
        }
    }

    false
}

pub fn is_archive_file<P: AsRef<Path>>(path: P) -> bool {
    is_supported_archive_extension(path.as_ref())
}

pub fn hash_file_contents<P: AsRef<Path>>(path: P) -> miette::Result<String> {
    let path = path.as_ref();

    trace!(file = ?path, "Calculating SHA256 checksum");

    let mut file = fs::open_file(path)?;
    let mut sha = Sha256::new();

    io::copy(&mut file, &mut sha).map_err(|error| FsError::Read {
        path: path.to_path_buf(),
        error,
    })?;

    let hash = format!("{:x}", sha.finalize());

    trace!(hash, "Calculated hash");

    Ok(hash)
}

pub fn read_json_file_with_lock<T: DeserializeOwned>(path: impl AsRef<Path>) -> miette::Result<T> {
    let path = path.as_ref();
    let content = fs::read_file_with_lock(path)?;

    let data: T = json::from_str(&content).map_err(|error| JsonError::ReadFile {
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

    let data = json::to_string_pretty(data).map_err(|error| JsonError::StringifyFile {
        path: path.to_path_buf(),
        error,
    })?;

    fs::write_file_with_lock(path, data)?;

    Ok(())
}
