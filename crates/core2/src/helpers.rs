use crate::error::ProtoError;
use cached::proc_macro::cached;
use starbase_utils::dirs::home_dir;
use std::{env, path::PathBuf};

pub fn get_root() -> miette::Result<PathBuf> {
    if let Ok(root) = env::var("PROTO_ROOT") {
        return Ok(root.into());
    }

    Ok(get_home_dir()?.join(".proto"))
}

pub fn get_home_dir() -> miette::Result<PathBuf> {
    Ok(home_dir().ok_or(ProtoError::MissingHomeDir)?)
}

pub fn get_bin_dir() -> miette::Result<PathBuf> {
    Ok(get_root()?.join("bin"))
}

pub fn get_temp_dir() -> miette::Result<PathBuf> {
    Ok(get_root()?.join("temp"))
}

pub fn get_tools_dir() -> miette::Result<PathBuf> {
    Ok(get_root()?.join("tools"))
}

pub fn get_plugins_dir() -> miette::Result<PathBuf> {
    Ok(get_root()?.join("plugins"))
}

// Aliases are words that map to version. For example, "latest" -> "1.2.3".
pub fn is_alias_name(value: &str) -> bool {
    value.chars().enumerate().all(|(i, c)| {
        if i == 0 {
            char::is_ascii_alphabetic(&c) && c != 'v' && c != 'V'
        } else {
            char::is_ascii_alphanumeric(&c) || c == '-'
        }
    })
}

pub fn add_v_prefix(value: &str) -> String {
    if value.starts_with('v') || value.starts_with('V') {
        return value.to_lowercase();
    }

    format!("v{value}")
}

pub fn remove_v_prefix(value: &str) -> String {
    if value.starts_with('v') || value.starts_with('V') {
        return value[1..].to_owned();
    }

    value.to_owned()
}

pub fn remove_space_after_gtlt(value: &str) -> String {
    let pattern = regex::Regex::new(r"([><]=?)\s+(\d)").unwrap();
    pattern.replace_all(value, "$1$2").to_string()
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
