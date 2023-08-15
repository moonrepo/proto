use extism_pdk::http::request;
use extism_pdk::*;
use once_cell::sync::Lazy;
use once_map::OnceMap;
use proto_pdk_api::{
    Environment, ExecCommandInput, ExecCommandOutput, HostArch, HostOS, PluginError,
    UserConfigSettings,
};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::vec;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

pub static FETCH_CACHE: Lazy<OnceMap<String, Vec<u8>>> = Lazy::new(OnceMap::new);

/// Fetch the provided request and deserialize the response as JSON.
pub fn fetch<R>(req: HttpRequest, body: Option<String>, cache: bool) -> anyhow::Result<R>
where
    R: DeserializeOwned,
{
    if cache {
        if let Some(body) = FETCH_CACHE.get(&req.url) {
            return Ok(json::from_slice(body)?);
        }
    }

    let res = request(&req, body)
        .map_err(|e| anyhow::anyhow!("Failed to make request to {}: {e}", req.url))?;

    // Only cache GET requests
    if cache && (req.method.is_none() || req.method.is_some_and(|m| m.to_uppercase() == "GET")) {
        FETCH_CACHE.insert(req.url, |_| res.body());
    }

    res.json()
}

/// Fetch the provided URL and deserialize the response as JSON.
pub fn fetch_url<R, U>(url: U) -> anyhow::Result<R>
where
    R: DeserializeOwned,
    U: AsRef<str>,
{
    fetch(HttpRequest::new(url.as_ref()), None, false)
}

/// Fetch the provided URL, deserialize the response as JSON,
/// and cache the response in memory for the duration of the WASM instance.
pub fn fetch_url_with_cache<R, U>(url: U) -> anyhow::Result<R>
where
    R: DeserializeOwned,
    U: AsRef<str>,
{
    fetch(HttpRequest::new(url.as_ref()), None, true)
}

/// Load all git tags from the provided remote URL.
/// The `git` binary must exist on the current machine.
pub fn load_git_tags<U>(url: U) -> anyhow::Result<Vec<String>>
where
    U: AsRef<str>,
{
    let output = unsafe {
        exec_command(Json(ExecCommandInput::pipe(
            "git",
            [
                "ls-remote",
                "--tags",
                "--sort",
                "version:refname",
                url.as_ref(),
            ],
        )))?
        .0
    };

    let mut tags: Vec<String> = vec![];

    for line in output.stdout.split('\n') {
        let parts = line.split('\t').collect::<Vec<_>>();

        if parts.len() < 2 {
            continue;
        }

        if let Some(tag) = parts[1].strip_prefix("refs/tags/") {
            tags.push(tag.to_owned());
        }
    }

    Ok(tags)
}

/// Return the name of the binary for the provided name and OS.
/// On Windows, will append ".exe", and keep as-is on other OS's.
pub fn format_bin_name(name: &str, os: HostOS) -> String {
    if os == HostOS::Windows {
        return format!("{}.exe", name);
    }

    name.to_owned()
}

/// Validate the current host OS and architecture against the
/// supported list of target permutations.
pub fn check_supported_os_and_arch(
    tool: &str,
    env: &Environment,
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

/// Return the loaded proto user configuration (`~/.proto/config.toml`). Does not include plugins!
pub fn get_proto_user_config() -> anyhow::Result<UserConfigSettings> {
    let config = config::get("proto_user_config").expect("Missing proto user configuration!");
    let config: UserConfigSettings = json::from_str(&config)?;

    Ok(config)
}
