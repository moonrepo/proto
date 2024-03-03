use extism_pdk::*;
use proto_pdk_api::{AnyResult, HostArch, HostEnvironment, HostLibc, HostOS, PluginError};
use rustc_hash::FxHashMap;
use serde::de::DeserializeOwned;

/// Validate the current host OS and architecture against the
/// supported list of target permutations.
pub fn check_supported_os_and_arch(
    tool: &str,
    env: &HostEnvironment,
    permutations: FxHashMap<HostOS, Vec<HostArch>>,
) -> AnyResult<()> {
    if let Some(archs) = permutations.get(&env.os) {
        if !archs.contains(&env.arch) {
            return Err(PluginError::UnsupportedTarget {
                tool: tool.to_owned(),
                arch: env.arch.to_string(),
                os: env.os.to_string(),
            }
            .into());
        }
    } else {
        return Err(PluginError::UnsupportedOS {
            tool: tool.to_owned(),
            os: env.os.to_string(),
        }
        .into());
    }

    Ok(())
}

/// Return a Rust target triple for the current host OS and architecture.
pub fn get_target_triple(env: &HostEnvironment, name: &str) -> Result<String, PluginError> {
    match env.os {
        HostOS::Linux => Ok(format!(
            "{}-unknown-linux-{}",
            env.arch.to_rust_arch(),
            if matches!(env.libc, HostLibc::Musl) {
                "musl"
            } else {
                "gnu"
            }
        )),
        HostOS::MacOS => Ok(format!("{}-apple-darwin", env.arch.to_rust_arch())),
        HostOS::Windows => Ok(format!("{}-pc-windows-msvc", env.arch.to_rust_arch())),
        _ => Err(PluginError::UnsupportedTarget {
            tool: name.into(),
            arch: env.arch.to_string(),
            os: env.os.to_string(),
        }),
    }
}

/// Get proto tool configuration that was configured in a `.prototools` file.
pub fn get_tool_config<T: Default + DeserializeOwned>() -> AnyResult<T> {
    let config: T = if let Some(value) = config::get("proto_tool_config")? {
        json::from_str(&value)?
    } else {
        T::default()
    };

    Ok(config)
}
