use extism_pdk::http::request;
use extism_pdk::*;
use proto_pdk_api::{ExecCommandInput, ExecCommandOutput};
use serde::de::DeserializeOwned;
use std::vec;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

/// Fetch the provided request and deserialize the response as JSON.
pub fn fetch<R>(req: HttpRequest) -> anyhow::Result<R>
where
    R: DeserializeOwned,
{
    request::<String>(&req, None)
        .map_err(|e| anyhow::anyhow!("Failed to make request to {}: {e}", req.url))?
        .json()
}

/// Fetch the provided URL and deserialize the response as JSON.
pub fn fetch_url<R, U>(url: U) -> anyhow::Result<R>
where
    R: DeserializeOwned,
    U: AsRef<str>,
{
    fetch(HttpRequest::new(url.as_ref()))
}

/// Load all git tags from the provided remote URL.
/// The `git` binary must exist on the current machine.
pub fn load_git_tags<U>(url: U) -> anyhow::Result<Vec<String>>
where
    U: AsRef<str>,
{
    let output = unsafe {
        exec_command(Json(ExecCommandInput::new(
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
