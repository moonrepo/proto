use crate::settings::ProtoToolVersionsSourceConfig;
use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use proto_pdk_api::{LoadToolVersionsInput, LoadToolVersionsOutput, select_index_versions};
use regex::Regex;
use rustc_hash::FxHashMap;
use starbase_styles::color;
use std::process::{Command, Stdio};
use std::time::Instant;
use tokio::runtime::Handle;
use tracing::{instrument, trace};
use warpgate::{HttpClient, WarpgateHttpClientError, host::HostData};

/// Data passed to proto specific host functions.
pub struct ProtoHostData {
    /// Shared host data.
    pub host: HostData,

    /// Tool override for where to source versions from.
    pub versions_source: Option<ProtoToolVersionsSourceConfig>,
}

/// Create a list of our built-in host function with warpgate provided host functions.
pub fn create_proto_host_functions(
    host: HostData,
    versions_source: Option<ProtoToolVersionsSourceConfig>,
) -> Vec<Function> {
    let mut functions = warpgate::host::create_host_functions(host.clone());

    functions.push(Function::new(
        "load_tool_versions",
        [ValType::I64],
        [ValType::I64],
        UserData::new(ProtoHostData {
            host,
            versions_source,
        }),
        load_tool_versions,
    ));

    functions
}

#[instrument(name = "host_func_load_tool_versions", skip_all)]
fn load_tool_versions(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<ProtoHostData>,
) -> Result<(), Error> {
    let instant = Instant::now();
    let input_raw: String = plugin.memory_get_val(&inputs[0])?;
    let input: LoadToolVersionsInput = serde_json::from_str(&input_raw)?;
    let uuid = plugin.id().to_string();
    let data = user_data.get()?;
    let data = data.lock().unwrap();

    let source = data.versions_source.as_ref();
    let git_url = source.and_then(|source| source.git_url.as_deref());
    let index_url = source.and_then(|source| source.index_url.as_deref());

    if git_url.is_some() && index_url.is_some() {
        return Err(Error::msg(
            "Tool versions source cannot configure both git-url and index-url",
        ));
    }

    let (source_type, source_url, override_used) = match (&input, index_url, git_url) {
        (LoadToolVersionsInput::Git { .. }, Some(index_url), _) => ("index", index_url, true),
        (LoadToolVersionsInput::Git { url, .. }, None, git_url) => (
            "git",
            git_url.unwrap_or(url),
            source.is_some_and(|source| {
                source.git_url.is_some()
                    || source.git_tag_pattern.is_some()
                    || source.version_pattern.is_some()
            }),
        ),
        (LoadToolVersionsInput::Index { .. }, _, Some(git_url)) => ("git", git_url, true),
        (LoadToolVersionsInput::Index { url, .. }, index_url, None) => (
            "index",
            index_url.unwrap_or(url),
            source.is_some_and(|source| {
                source.index_url.is_some()
                    || source.index_version_key.is_some()
                    || source.version_pattern.is_some()
            }),
        ),
    };

    trace!(
        plugin = &uuid,
        input = %input_raw,
        source_type,
        source_url, override_used,
        "Calling host function {}",
        color::label("load_tool_versions"),
    );

    let output = match &input {
        LoadToolVersionsInput::Git { url, tag_pattern } => {
            if let Some(index_url) = index_url {
                load_index_source(
                    &data.host,
                    source,
                    IndexRequest {
                        url: index_url,
                        headers: None,
                        include_body: false,
                        timeout: plugin.time_remaining(),
                    },
                    None,
                )?
            } else {
                LoadToolVersionsOutput {
                    versions: load_git_source(
                        &data.host,
                        source,
                        git_url.unwrap_or(url),
                        tag_pattern.as_deref(),
                    )?,
                    ..Default::default()
                }
            }
        }
        LoadToolVersionsInput::Index {
            url,
            version_key,
            headers,
            include_body,
        } => {
            if let Some(git_url) = git_url {
                LoadToolVersionsOutput {
                    versions: load_git_source(&data.host, source, git_url, None)?,
                    ..Default::default()
                }
            } else {
                // The plugin's headers and body only apply to its own URL, as a
                // configured URL is another host, with another index format
                let own_url = index_url.is_none();

                load_index_source(
                    &data.host,
                    source,
                    IndexRequest {
                        url: index_url.unwrap_or(url),
                        headers: own_url.then_some(headers),
                        include_body: own_url && *include_body,
                        timeout: plugin.time_remaining(),
                    },
                    version_key.as_deref(),
                )?
            }
        }
    };

    trace!(
        plugin = &uuid,
        input = %input_raw,
        source_type,
        source_url, override_used,
        "Called host function {} in {:?}",
        color::label("load_tool_versions"),
        instant.elapsed()
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&output)?)?;

    Ok(())
}

fn load_git_source(
    data: &HostData,
    source: Option<&ProtoToolVersionsSourceConfig>,
    url: &str,
    input_pattern: Option<&str>,
) -> Result<Vec<String>, Error> {
    let pattern = source
        .and_then(|source| source.git_tag_pattern.as_deref())
        .or_else(|| source.and_then(|source| source.version_pattern.as_deref()))
        .or(input_pattern);

    filter_versions(load_tool_versions_from_git(data, url)?, pattern, "Git tag")
}

fn load_tool_versions_from_git(data: &HostData, url: &str) -> Result<Vec<String>, Error> {
    let mut command = Command::new("git");
    command
        .args(["ls-remote", "--tags", "--sort", "version:refname", url])
        .current_dir(&data.working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = command.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        return Err(Error::msg(format!(
            "Failed to load Git tags from <url>{url}</url>, as <shell>git ls-remote</shell> returned a {} exit code.{}",
            output.status.code().unwrap_or(1),
            if stderr.trim().is_empty() {
                String::new()
            } else {
                format!("\n<mutedlight>{}</mutedlight>", stderr.trim())
            }
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut versions = vec![];

    for line in stdout.lines() {
        if line.ends_with("^{}") {
            continue;
        }

        if let Some((_, reference)) = line.split_once('\t')
            && let Some(version) = reference.strip_prefix("refs/tags/")
        {
            versions.push(version.to_owned());
        }
    }

    Ok(versions)
}

struct IndexRequest<'a> {
    url: &'a str,
    headers: Option<&'a FxHashMap<String, String>>,
    include_body: bool,
    timeout: Option<std::time::Duration>,
}

fn load_index_source(
    data: &HostData,
    source: Option<&ProtoToolVersionsSourceConfig>,
    request: IndexRequest,
    input_path: Option<&str>,
) -> Result<LoadToolVersionsOutput, Error> {
    let version_path = source
        .and_then(|source| source.index_version_key.as_ref())
        .map(|key| format!("*.{key}"))
        .or_else(|| input_path.map(str::to_owned))
        .unwrap_or_else(|| "*.version".to_owned());

    let mut output = load_tool_versions_from_index(data, request, &version_path)?;

    output.versions = filter_versions(
        output.versions,
        source.and_then(|source| source.version_pattern.as_deref()),
        "Index version",
    )?;

    Ok(output)
}

fn load_tool_versions_from_index(
    data: &HostData,
    input: IndexRequest,
    version_path: &str,
) -> Result<LoadToolVersionsOutput, Error> {
    let url = input.url;
    let (status, body) = Handle::current().block_on(async {
        let mut request = data.http_client.get(url);

        if let Some(timeout) = input.timeout {
            request = request.timeout(timeout);
        }

        for (name, value) in input.headers.into_iter().flatten() {
            request = request.header(name, value);
        }

        let response = request
            .send()
            .await
            .map_err(|error| HttpClient::map_error(url.to_owned(), error))?;
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|error| WarpgateHttpClientError::Http {
                url: url.to_owned(),
                error: Box::new(error),
            })?;

        Ok::<_, WarpgateHttpClientError>((status, body))
    })?;

    if !status.is_success() {
        return Err(Error::msg(format!(
            "Failed to load tool versions from <url>{url}</url> <mutedlight>({})</mutedlight>",
            status.as_u16()
        )));
    }

    let index: serde_json::Value = serde_json::from_slice(&body)?;

    Ok(LoadToolVersionsOutput {
        versions: select_index_versions(&index, version_path)?,
        body: input
            .include_body
            .then(|| String::from_utf8_lossy(&body).into_owned()),
    })
}

fn filter_versions(
    versions: Vec<String>,
    pattern: Option<&str>,
    source: &str,
) -> Result<Vec<String>, Error> {
    let Some(pattern) = pattern else {
        return Ok(versions);
    };

    let regex = Regex::new(pattern)
        .map_err(|error| Error::msg(format!("Invalid {source} pattern {pattern:?}: {error}")))?;

    Ok(versions
        .into_iter()
        .filter(|version| regex.is_match(version))
        .collect())
}
