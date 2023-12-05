use crate::exec_command;
use extism_pdk::http::request;
use extism_pdk::*;
use proto_pdk_api::{
    ExecCommandInput, ExecCommandOutput, HostArch, HostEnvironment, HostOS, PluginError,
};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::vec;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

/// Fetch the provided request and return a response object.
pub fn fetch(req: HttpRequest, body: Option<String>) -> anyhow::Result<HttpResponse> {
    debug!("Fetching URL {}", req.url);

    request(&req, body).map_err(|e| anyhow::anyhow!("Failed to make request to {}: {e}", req.url))
}

/// Fetch the provided URL and deserialize the response as JSON.
pub fn fetch_url<R, U>(url: U) -> anyhow::Result<R>
where
    R: DeserializeOwned,
    U: AsRef<str>,
{
    fetch(HttpRequest::new(url.as_ref()), None)?.json()
}

/// Fetch the provided URL and return the text response.
pub fn fetch_url_text<U>(url: U) -> anyhow::Result<String>
where
    U: AsRef<str>,
{
    String::from_bytes(fetch(HttpRequest::new(url.as_ref()), None)?.body())
}

/// Fetch the provided URL, deserialize the response as JSON,
/// and cache the response in memory for subsequent WASM function calls.
pub fn fetch_url_with_cache<R, U>(url: U) -> anyhow::Result<R>
where
    R: DeserializeOwned,
    U: AsRef<str>,
{
    let url = url.as_ref();
    let req = HttpRequest::new(url);

    // Only cache GET requests
    let cache = req.method.is_none()
        || req
            .method
            .as_ref()
            .is_some_and(|m| m.to_uppercase() == "GET");

    if cache {
        if let Some(body) = var::get::<Vec<u8>>(url)? {
            debug!("Reading URL {} from cache (len = {})", url, body.len());

            return Ok(json::from_slice(&body)?);
        }
    }

    let res = fetch(req, None)?;

    if cache {
        let body = res.body();

        debug!("Writing URL {} to cache (len = {})", url, body.len());

        var::set(url, body)?;
    }

    res.json()
}

/// Load all git tags from the provided remote URL.
/// The `git` binary must exist on the current machine.
pub fn load_git_tags<U>(url: U) -> anyhow::Result<Vec<String>>
where
    U: AsRef<str>,
{
    let url = url.as_ref();

    debug!("Loading Git tags from remote {}", url);

    let output = exec_command!(
        pipe,
        "git",
        ["ls-remote", "--tags", "--sort", "version:refname", url]
    );

    let mut tags: Vec<String> = vec![];

    if output.exit_code != 0 {
        debug!("Failed to load Git tags");

        return Ok(tags);
    }

    for line in output.stdout.split('\n') {
        // https://superuser.com/questions/1445823/what-does-mean-in-the-tags
        if line.ends_with("^{}") {
            continue;
        }

        let parts = line.split('\t').collect::<Vec<_>>();

        if parts.len() < 2 {
            continue;
        }

        if let Some(tag) = parts[1].strip_prefix("refs/tags/") {
            tags.push(tag.to_owned());
        }
    }

    debug!("Loaded {} tags", tags.len());

    Ok(tags)
}

/// Return the name of the binary for the provided name and OS.
/// On Windows, will append ".exe", and keep as-is on other OS's.
#[deprecated]
pub fn format_bin_name<T: AsRef<str>>(name: T, os: HostOS) -> String {
    if os == HostOS::Windows {
        return format!("{}.exe", name.as_ref());
    }

    name.as_ref().to_owned()
}

/// Validate the current host OS and architecture against the
/// supported list of target permutations.
pub fn check_supported_os_and_arch(
    tool: &str,
    env: &HostEnvironment,
    permutations: HashMap<HostOS, Vec<HostArch>>,
) -> anyhow::Result<()> {
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

/// Check whether a command exists or not on the host machine.
pub fn command_exists(env: &HostEnvironment, command: &str) -> bool {
    debug!("Checking if command `{}` exists on the host", command);

    let result = if env.os == HostOS::Windows {
        let line = format!("Get-Command {command}");

        exec_command!(raw, "powershell", ["-Command", &line])
    } else {
        exec_command!(raw, "which", [command])
    };

    if result.is_ok_and(|res| res.0.exit_code == 0) {
        debug!("Does exist");

        return true;
    }

    debug!("Does NOT exist");

    false
}

/// Detect whether the current OS is utilizing musl instead of gnu.
pub fn is_musl(env: &HostEnvironment) -> bool {
    if !env.os.is_unix() || env.os.is_mac() {
        return false;
    }

    debug!("Checking if host is using musl");

    let mut value = "".to_owned();

    if let Ok(res) = exec_command!(raw, "ldd", ["--version"]) {
        if res.0.exit_code == 0 {
            value = res.0.stdout.to_lowercase();
        }
    }

    if value.is_empty() {
        if let Ok(res) = exec_command!(raw, "uname") {
            if res.0.exit_code == 0 {
                value = res.0.stdout.to_lowercase();
            }
        }
    }

    if value.contains("musl") || value.contains("alpine") {
        debug!("Is using musl");

        return true;
    }

    debug!("Is NOT using musl");

    false
}

/// Return a Rust target triple for the current host OS and architecture.
pub fn get_target_triple(env: &HostEnvironment, name: &str) -> Result<String, PluginError> {
    match env.os {
        HostOS::Linux => Ok(format!(
            "{}-unknown-linux-{}",
            env.arch.to_rust_arch(),
            if is_musl(env) { "musl" } else { "gnu" }
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

/// Get the tool ID for the current WASM plugin.
pub fn get_tool_id() -> String {
    config::get("proto_tool_id").expect("Missing tool ID!")
}

/// Get tool configuration for the current WASM plugin that was configured in a `.prototools` file.
pub fn get_tool_config<T: Default + DeserializeOwned>() -> anyhow::Result<T> {
    let config: T = if let Some(value) = config::get("proto_tool_config") {
        json::from_str(&value)?
    } else {
        T::default()
    };

    Ok(config)
}

/// Return information about proto and the host environment.
pub fn get_proto_environment() -> anyhow::Result<HostEnvironment> {
    let config = config::get("proto_environment").expect("Missing proto environment!");
    let config: HostEnvironment = json::from_str(&config)?;

    Ok(config)
}

/// Return the loaded proto user configuration (`~/.proto/.prototools`). Does not include plugins!
#[allow(deprecated)]
#[deprecated]
pub fn get_proto_user_config() -> anyhow::Result<json::Value> {
    if let Some(config) = config::get("proto_user_config") {
        return Ok(json::from_str(&config)?);
    }

    Ok(json::Value::default())
}
