use extism_pdk::http::request;
use extism_pdk::HttpRequest;
use human_sort::compare;
use serde::de::DeserializeOwned;
use std::process::Command;

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
    let url = url.as_ref();

    let output = Command::new("git")
        .args(["ls-remote", "--tags", "--sort", "version:refname", url])
        .output()
        .map_err(|e| anyhow::anyhow!("Could not list git tags: {e}"))?;

    let raw = String::from_utf8(output.stdout)
        .map_err(|e| anyhow::anyhow!("Could not parse git tags: {e}"))?;

    let mut tags: Vec<String> = vec![];

    for line in raw.split('\n') {
        let parts: Vec<&str> = line.split('\t').collect();

        if parts.len() < 2 {
            continue;
        }

        if let Some(tag) = parts[1].strip_prefix("refs/tags/") {
            tags.push(tag.to_owned());
        }
    }

    tags.sort_by(|a, d| compare(a, d));

    Ok(tags)
}
