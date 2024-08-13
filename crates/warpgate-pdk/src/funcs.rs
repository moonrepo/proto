use crate::api::populate_send_request_output;
use crate::{exec_command, send_request};
use extism_pdk::http::request;
use extism_pdk::*;
use serde::de::DeserializeOwned;
use std::vec;
use warpgate_api::{
    anyhow, AnyResult, ExecCommandInput, ExecCommandOutput, HostEnvironment, HostOS,
    SendRequestInput, SendRequestOutput, TestEnvironment,
};

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
}

/// Fetch the provided request and return a response object.
#[deprecated(note = "Use `fetch_*` instead.")]
pub fn fetch(req: HttpRequest, body: Option<String>) -> AnyResult<HttpResponse> {
    debug!("Fetching <url>{}</url>", req.url);

    request(&req, body)
        .map_err(|error| error.context(format!("Failed to make request to <url>{}</url>", req.url)))
}

/// Fetch the provided URL and deserialize the response as JSON.
#[allow(deprecated)]
#[deprecated(note = "Use `fetch_json` instead.")]
pub fn fetch_url<R, U>(url: U) -> AnyResult<R>
where
    R: DeserializeOwned,
    U: AsRef<str>,
{
    fetch(HttpRequest::new(url.as_ref()), None)?.json()
}

/// Fetch the provided URL and deserialize the response as bytes.
#[allow(deprecated)]
#[deprecated(note = "Use `fetch_bytes` instead.")]
pub fn fetch_url_bytes<U>(url: U) -> AnyResult<Vec<u8>>
where
    U: AsRef<str>,
{
    Ok(fetch(HttpRequest::new(url.as_ref()), None)?.body())
}

/// Fetch the provided URL and return the text response.
#[allow(deprecated)]
#[deprecated(note = "Use `fetch_text` instead.")]
pub fn fetch_url_text<U>(url: U) -> AnyResult<String>
where
    U: AsRef<str>,
{
    String::from_bytes(&fetch_url_bytes(url)?)
}

/// Fetch the provided URL, deserialize the response as JSON,
/// and cache the response in memory for subsequent WASM function calls.
#[allow(deprecated)]
#[deprecated(note = "Use `fetch_*` instead.")]
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

fn do_fetch<U>(url: U) -> AnyResult<SendRequestOutput>
where
    U: AsRef<str>,
{
    let url = url.as_ref();
    let response = send_request!(url);
    let status = response.status;

    if status != 200 {
        let body = response.text()?;

        debug!(
            "Response body for <url>{}</url>: <muted>{}</muted>",
            url, body
        );

        return Err(anyhow!(
            "Failed to request <url>{url}</url> <mutedlight>({})</mutedlight>",
            status
        ));
    }

    if response.body.is_empty() {
        return Err(anyhow!("Invalid response from <url>{url}</url>, no body"));
    }

    Ok(response)
}

/// Fetch the provided URL and return the response as bytes.
pub fn fetch_bytes<U>(url: U) -> AnyResult<Vec<u8>>
where
    U: AsRef<str>,
{
    Ok(do_fetch(url)?.body)
}

/// Fetch the provided URL and deserialize the response as JSON.
pub fn fetch_json<U, R>(url: U) -> AnyResult<R>
where
    U: AsRef<str>,
    R: DeserializeOwned,
{
    do_fetch(url)?.json()
}

/// Fetch the provided URL and return the response as text.
pub fn fetch_text<U>(url: U) -> AnyResult<String>
where
    U: AsRef<str>,
{
    do_fetch(url)?.text()
}

/// Load all Git tags from the provided remote URL.
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
        exec_command!(
            raw,
            "powershell",
            ["-Command", format!("Get-Command {command}").as_str()]
        )
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
