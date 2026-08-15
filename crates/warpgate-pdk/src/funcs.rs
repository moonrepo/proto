use crate::api::populate_send_request_output;
use crate::{download_file, exec_command, send_request};
use extism_pdk::*;
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::vec;
use warpgate_api::{
    AnyResult, DownloadFileInput, DownloadFileOutput, ExecCommandInput, ExecCommandOutput,
    HostEnvironment, HostOS, Id, SendRequestInput, SendRequestOutput, TestEnvironment, VirtualPath,
    anyhow,
};

#[host_fn]
extern "ExtismHost" {
    fn download_file(input: Json<DownloadFileInput>) -> Json<DownloadFileOutput>;
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
    fn get_env_var(key: String) -> String;
    fn set_env_var(name: String, value: String);
}

/// Fetch the requested input and return a response.
pub fn fetch(input: SendRequestInput) -> AnyResult<SendRequestOutput> {
    let url = input.url.clone();
    let response = send_request!(input, input);
    let status = response.status;

    if !(200..300).contains(&status) {
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
    Ok(fetch(SendRequestInput::new(url))?.body)
}

/// Fetch the provided URL and deserialize the response as JSON.
pub fn fetch_json<U, R>(url: U) -> AnyResult<R>
where
    U: AsRef<str>,
    R: DeserializeOwned,
{
    fetch(SendRequestInput::new(url))?.json()
}

/// Fetch the provided URL and return the response as text.
pub fn fetch_text<U>(url: U) -> AnyResult<String>
where
    U: AsRef<str>,
{
    fetch(SendRequestInput::new(url))?.text()
}

/// Download a file with the requested input, and save it to the
/// destination file on the host machine.
pub fn download(input: DownloadFileInput) -> AnyResult<DownloadFileOutput> {
    debug!(
        "Downloading <url>{}</url> to <path>{}</path>",
        input.url, input.file
    );

    Ok(download_file!(input, input))
}

/// Download a file from the provided URL, and save it to the
/// destination file on the host machine.
pub fn download_from_url<U, F>(url: U, file: F) -> AnyResult<DownloadFileOutput>
where
    U: AsRef<str>,
    F: AsRef<VirtualPath>,
{
    download(DownloadFileInput::new(url, file.as_ref()))
}

/// Execute a command on the host with the provided input.
pub fn exec(input: ExecCommandInput) -> AnyResult<ExecCommandOutput> {
    Ok(exec_command!(input, input))
}

/// Execute a command on the host and capture its output (pipe).
pub fn exec_captured<C, I, A>(command: C, args: I) -> AnyResult<ExecCommandOutput>
where
    C: AsRef<str>,
    I: IntoIterator<Item = A>,
    A: AsRef<str>,
{
    exec(ExecCommandInput::pipe(command, args))
}

/// Execute a command on the host and stream its output to the console (inherit).
pub fn exec_streamed<C, I, A>(command: C, args: I) -> AnyResult<ExecCommandOutput>
where
    C: AsRef<str>,
    I: IntoIterator<Item = A>,
    A: AsRef<str>,
{
    exec(ExecCommandInput::inherit(command, args))
}

/// Load all Git tags from the provided remote URL.
/// The `git` executable must exist on the host machine.
pub fn load_git_tags<U>(url: U) -> AnyResult<Vec<String>>
where
    U: AsRef<str>,
{
    let url = url.as_ref();

    debug!("Loading Git tags from remote <url>{}</url>", url);

    let mut tags: Vec<String> = vec![];
    let output = exec_captured(
        "git",
        ["ls-remote", "--tags", "--sort", "version:refname", url],
    )?;

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
        exec_captured(
            "powershell",
            ["-Command", format!("Get-Command {command}").as_str()],
        )
    } else {
        exec_captured("which", [command])
    };

    if result.is_ok_and(|res| res.exit_code == 0) {
        debug!("Command does exist");

        return true;
    }

    debug!("Command does NOT exist");

    false
}

/// Return the value of an environment variable on the host machine.
pub fn get_host_env_var<K>(key: K) -> AnyResult<Option<String>>
where
    K: AsRef<str>,
{
    let inner = unsafe { get_env_var(key.as_ref().into())? };

    Ok(if inner.is_empty() { None } else { Some(inner) })
}

/// Set the value of an environment variable on the host machine.
pub fn set_host_env_var<K, V>(key: K, value: V) -> AnyResult<()>
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    unsafe { set_env_var(key.as_ref().into(), value.as_ref().into())? };

    Ok(())
}

/// Append paths to the `PATH` environment variable on the host machine.
pub fn add_host_paths<I, P>(paths: I) -> AnyResult<()>
where
    I: IntoIterator<Item = P>,
    P: AsRef<str>,
{
    let paths = paths
        .into_iter()
        .map(|p| p.as_ref().to_owned())
        .collect::<Vec<_>>();

    set_host_env_var("PATH", paths.join(":"))
}

/// Return the ID for the current plugin.
pub fn get_plugin_id() -> AnyResult<Id> {
    Ok(Id::raw(
        config::get("plugin_id")?.expect("Missing plugin ID!"),
    ))
}

/// Return information about the host environment.
pub fn get_host_environment() -> AnyResult<&'static HostEnvironment> {
    static HOST_ENVIRONMENT: OnceLock<HostEnvironment> = OnceLock::new();

    if HOST_ENVIRONMENT.get().is_none() {
        let config = config::get("host_environment")?.expect("Missing host environment!");
        let _ = HOST_ENVIRONMENT.set(json::from_str(&config)?);
    }

    Ok(HOST_ENVIRONMENT.get_or_init(HostEnvironment::default))
}

/// Return information about the testing environment.
pub fn get_test_environment() -> AnyResult<Option<&'static TestEnvironment>> {
    static TEST_ENVIRONMENT: OnceLock<Option<TestEnvironment>> = OnceLock::new();

    if TEST_ENVIRONMENT.get().is_none() {
        if let Some(config) = config::get("test_environment")? {
            let _ = TEST_ENVIRONMENT.set(json::from_str(&config)?);
        }
    }

    Ok(TEST_ENVIRONMENT.get_or_init(|| None).as_ref())
}

/// Return a mapping of virtual paths, from host to guest paths.
pub fn get_host_to_guest_paths() -> AnyResult<&'static Vec<(PathBuf, PathBuf)>> {
    static PATHS_LIST: OnceLock<Vec<(PathBuf, PathBuf)>> = OnceLock::new();

    if PATHS_LIST.get().is_none() {
        if let Some(config) = config::get("virtual_paths")? {
            let _ = PATHS_LIST.set(json::from_str(&config)?);
        }
    }

    Ok(PATHS_LIST.get_or_init(Vec::new))
}
