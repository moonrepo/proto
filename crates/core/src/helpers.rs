use crate::error::ProtoError;
use cached::proc_macro::cached;
use miette::IntoDiagnostic;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};
use starbase_archive::is_supported_archive_extension;
use starbase_utils::dirs::home_dir;
use starbase_utils::fs::{self, FsError};
use starbase_utils::json::{self, JsonError};
use std::path::Path;
use std::time::Instant;
use std::{env, path::PathBuf};
use std::{io, thread};
use tracing::trace;

pub static ENV_VAR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$([A-Z0-9_]+)").unwrap());

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

pub fn get_shims_dir() -> miette::Result<PathBuf> {
    Ok(get_proto_home()?.join("shims"))
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

    trace!("Checking for an internet connection");

    use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let timeout = Duration::new(1, 0);
    let is_online = |addresses: Vec<SocketAddr>| -> bool {
        for address in addresses {
            trace!("Resolving {address}");

            if let Ok(stream) = TcpStream::connect_timeout(&address, timeout) {
                let _ = stream.shutdown(Shutdown::Both);

                trace!("Online!");

                return true;
            }
        }

        false
    };

    // Check these first as they do not need to resolve IP addresses!
    if is_online(vec![
        // Cloudflare DNS: https://1.1.1.1/dns/
        SocketAddr::from(([1, 1, 1, 1], 53)),
        SocketAddr::from(([1, 0, 0, 1], 53)),
        // Google DNS: https://developers.google.com/speed/public-dns
        SocketAddr::from(([8, 8, 8, 8], 53)),
        SocketAddr::from(([8, 8, 4, 4], 53)),
    ]) {
        return false;
    }

    // Check these second as they need to resolve IP addresses,
    // which adds unnecessary time and overhead that can't be
    // controlled with a native timeout.
    let hosts = vec![
        "clients3.google.com:80",
        "detectportal.firefox.com:80",
        "google.com:80",
    ];

    'outer: for host in hosts {
        let handle = thread::spawn(|| host.to_socket_addrs().ok());
        let start = Instant::now();

        // If resolving an IP from the host takes longer than a second,
        // ignore the process and move onto the next host!
        'inner: loop {
            let elapsed = start.elapsed();

            if elapsed >= timeout {
                continue 'outer;
            } else if handle.is_finished() {
                break 'inner;
            }
        }

        if let Ok(Some(addrs)) = handle.join() {
            if is_online(addrs.collect()) {
                return false;
            }
        }
    }

    // if let Ok(addrs) = "clients3.google.com:80".to_socket_addrs() {
    //     if is_online(addrs.collect()) {
    //         return false;
    //     }
    // }

    // if let Ok(addrs) = "detectportal.firefox.com:80".to_socket_addrs() {
    //     if is_online(addrs.collect()) {
    //         return false;
    //     }
    // }

    // if let Ok(addrs) = "google.com:80".to_socket_addrs() {
    //     if is_online(addrs.collect()) {
    //         return false;
    //     }
    // }

    trace!("Offline!!!");

    true
}

pub fn is_cache_enabled() -> bool {
    env::var("PROTO_CACHE").map_or(true, |value| {
        value != "0" && value != "false" && value != "no" && value != "off"
    })
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
