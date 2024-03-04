use crate::exec_command;
use extism_pdk::http::request;
use extism_pdk::*;
use serde::de::DeserializeOwned;
use std::vec;
use warpgate_api::{
    AnyResult, ExecCommandInput, ExecCommandOutput, HostEnvironment, HostOS, TestEnvironment,
};

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

/// Fetch the provided request and return a response object.
pub fn fetch(req: HttpRequest, body: Option<String>) -> AnyResult<HttpResponse> {
    debug!("Fetching <url>{}</url>", req.url);

    request(&req, body)
        .map_err(|error| error.context(format!("Failed to make request to <url>{}</url>", req.url)))
}

/// Fetch the provided URL and deserialize the response as JSON.
pub fn fetch_url<R, U>(url: U) -> AnyResult<R>
where
    R: DeserializeOwned,
    U: AsRef<str>,
{
    fetch(HttpRequest::new(url.as_ref()), None)?.json()
}

/// Fetch the provided URL and deserialize the response as bytes.
pub fn fetch_url_bytes<U>(url: U) -> AnyResult<Vec<u8>>
where
    U: AsRef<str>,
{
    Ok(fetch(HttpRequest::new(url.as_ref()), None)?.body())
}

/// Fetch the provided URL and return the text response.
pub fn fetch_url_text<U>(url: U) -> AnyResult<String>
where
    U: AsRef<str>,
{
    String::from_bytes(&fetch_url_bytes(url)?)
}

/// Fetch the provided URL, deserialize the response as JSON,
/// and cache the response in memory for subsequent WASM function calls.
pub fn fetch_url_with_cache<R, U>(url: U) -> AnyResult<R>
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
            debug!(
                "Reading <url>{}</url> from cache <mutedlight>(length = {})</mutedlight>",
                url,
                body.len()
            );

            return Ok(json::from_slice(&body)?);
        }
    }

    let res = fetch(req, None)?;

    if cache {
        let body = res.body();

        debug!(
            "Writing <url>{}</url> to cache <mutedlight>(length = {})</mutedlight>",
            url,
            body.len()
        );

        var::set(url, body)?;
    }

    res.json()
}

/// Load all git tags from the provided remote URL.
/// The `git` binary must exist on the current machine.
pub fn load_git_tags<U>(url: U) -> AnyResult<Vec<String>>
where
    U: AsRef<str>,
{
    let url = url.as_ref();

    debug!("Loading Git tags from remote <url>{}</url>", url);

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

    debug!("Loaded {} Git tags", tags.len());

    Ok(tags)
}

/// Check whether a command exists or not on the host machine.
pub fn command_exists(env: &HostEnvironment, command: &str) -> bool {
    debug!(
        "Checking if command <shell>{}</shell> exists on the host",
        command
    );

    let result = if env.os == HostOS::Windows {
        let line = format!("Get-Command {command}");

        exec_command!(raw, "powershell", ["-Command", &line])
    } else {
        exec_command!(raw, "which", [command])
    };

    if result.is_ok_and(|res| res.0.exit_code == 0) {
        debug!("Command does exist");

        return true;
    }

    debug!("Command does NOT exist");

    false
}

/// Detect whether the current OS is utilizing musl instead of gnu.
#[deprecated]
pub fn is_musl(env: &HostEnvironment) -> bool {
    if !env.os.is_unix() || env.os.is_mac() {
        return false;
    }

    debug!("Checking if host is using musl");

    let mut value = "".to_owned();

    if command_exists(env, "ldd") {
        if let Ok(res) = exec_command!(raw, "ldd", ["--version"]) {
            if res.0.exit_code == 0 {
                value = res.0.stdout.to_lowercase();
            } else if res.0.exit_code == 1 {
                // ldd on apline returns stderr with a 1 exit code
                value = res.0.stderr.to_lowercase();
            }
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
        debug!("Host is using musl");

        return true;
    }

    debug!("Host is NOT using musl");

    false
}

/// Return the ID for the current plugin.
pub fn get_plugin_id() -> AnyResult<String> {
    Ok(config::get("plugin_id")?.expect("Missing plugin ID!"))
}

/// Return information about the host environment.
pub fn get_host_environment() -> AnyResult<HostEnvironment> {
    let config = config::get("host_environment")?.expect("Missing host environment!");
    let config: HostEnvironment = json::from_str(&config)?;

    Ok(config)
}

/// Return information about the testing environment.
pub fn get_test_environment() -> AnyResult<Option<TestEnvironment>> {
    if let Some(config) = config::get("test_environment")? {
        return Ok(json::from_str(&config)?);
    }

    Ok(None)
}
