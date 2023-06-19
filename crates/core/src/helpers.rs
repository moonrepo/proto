use crate::errors::ProtoError;
use cached::proc_macro::cached;
use dirs::home_dir;
use std::process::Command;
use std::{env, path::PathBuf};

pub fn get_root() -> Result<PathBuf, ProtoError> {
    if let Ok(root) = env::var("PROTO_ROOT") {
        return Ok(root.into());
    }

    Ok(get_home_dir()?.join(".proto"))
}

pub fn get_home_dir() -> Result<PathBuf, ProtoError> {
    home_dir().ok_or(ProtoError::MissingHomeDir)
}

pub fn get_bin_dir() -> Result<PathBuf, ProtoError> {
    Ok(get_root()?.join("bin"))
}

pub fn get_temp_dir() -> Result<PathBuf, ProtoError> {
    Ok(get_root()?.join("temp"))
}

pub fn get_tools_dir() -> Result<PathBuf, ProtoError> {
    Ok(get_root()?.join("tools"))
}

pub fn get_plugins_dir() -> Result<PathBuf, ProtoError> {
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

#[tracing::instrument]
pub fn has_command(command: &str) -> bool {
    Command::new(if cfg!(windows) {
        "Get-Command"
    } else {
        "which"
    })
    .arg(command)
    .output()
    .map(|output| output.status.success() && !output.stdout.is_empty())
    .unwrap_or(false)
}

#[cached]
pub fn is_musl() -> bool {
    let Ok(output) = Command::new("ldd").arg("--version").output() else {
        return false;
    };

    String::from_utf8_lossy(&output.stdout).contains("musl")
}

pub fn is_cache_enabled() -> bool {
    env::var("PROTO_CACHE").map_or(true, |value| {
        value != "0" && value != "false" && value != "no" && value != "off"
    })
}
