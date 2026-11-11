use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use proto_pdk_api::{LoadToolVersionsInput, LoadToolVersionsOutput};
use regex::Regex;
use std::process::{Command, Stdio};
use tokio::runtime::Handle;
use tracing::{debug, instrument};
use warpgate::{HttpClient, WarpgateHttpClientError, host::HostData};

#[derive(Clone, Debug)]
pub(crate) struct ToolVersionsSource {
    pub version_pattern: Option<String>,
    pub index_url: Option<String>,
    pub index_version_key: Option<String>,
    pub git_url: Option<String>,
    pub git_tag_pattern: Option<String>,
}

struct ProtoHostData {
    host: HostData,
    versions_source: Option<ToolVersionsSource>,
}

pub(crate) fn create_proto_host_functions(
    host: HostData,
    versions_source: Option<ToolVersionsSource>,
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
    let input_raw: String = plugin.memory_get_val(&inputs[0])?;
    let input: LoadToolVersionsInput = serde_json::from_str(&input_raw)?;
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

    debug!(
        source_type,
        source_url, override_used, "Loading tool versions"
    );

    let versions = match input {
        LoadToolVersionsInput::Git { url, tag_pattern } => {
            if let Some(index_url) = index_url {
                load_index_source(&data.host, source, index_url, None, plugin.time_remaining())?
            } else {
                load_git_source(
                    &data.host,
                    source,
                    git_url.unwrap_or(&url),
                    tag_pattern.as_deref(),
                )?
            }
        }
        LoadToolVersionsInput::Index { url, version_path } => {
            if let Some(git_url) = git_url {
                load_git_source(&data.host, source, git_url, None)?
            } else {
                load_index_source(
                    &data.host,
                    source,
                    index_url.unwrap_or(&url),
                    version_path.as_deref(),
                    plugin.time_remaining(),
                )?
            }
        }
    };

    plugin.memory_set_val(
        &mut outputs[0],
        serde_json::to_string(&LoadToolVersionsOutput { versions })?,
    )?;

    Ok(())
}

fn load_git_source(
    data: &HostData,
    source: Option<&ToolVersionsSource>,
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

fn load_index_source(
    data: &HostData,
    source: Option<&ToolVersionsSource>,
    url: &str,
    input_path: Option<&str>,
    timeout: Option<std::time::Duration>,
) -> Result<Vec<String>, Error> {
    let version_path = source
        .and_then(|source| source.index_version_key.as_ref())
        .map(|key| format!("*.{key}"))
        .or_else(|| input_path.map(str::to_owned))
        .unwrap_or_else(|| "*.version".to_owned());

    let versions = load_tool_versions_from_index(data, url, &version_path, timeout)?;

    filter_versions(
        versions,
        source.and_then(|source| source.version_pattern.as_deref()),
        "Index version",
    )
}

fn load_tool_versions_from_index(
    data: &HostData,
    url: &str,
    version_path: &str,
    timeout: Option<std::time::Duration>,
) -> Result<Vec<String>, Error> {
    let (status, body) = Handle::current().block_on(async {
        let mut request = data.http_client.get(url);

        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
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
    let values = select_json_path(&index, version_path)?;
    let values = if values.len() == 1 {
        values[0]
            .as_array()
            .map(|array| array.iter().collect())
            .unwrap_or(values)
    } else {
        values
    };

    values
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                Error::msg(format!(
                    "Tool versions at path {version_path:?} must be a string or contain only strings"
                ))
            })
        })
        .collect()
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

fn select_json_path<'a>(
    root: &'a serde_json::Value,
    path: &str,
) -> Result<Vec<&'a serde_json::Value>, Error> {
    enum Segment {
        Key(String),
        Index(usize),
        Wildcard,
    }

    let mut segments = vec![];
    let mut chars = path.chars().peekable();

    if chars.peek() == Some(&'$') {
        chars.next();
    }

    while let Some(character) = chars.next() {
        match character {
            '.' => {
                let mut key = String::new();

                while let Some(&next) = chars.peek() {
                    if matches!(next, '.' | '[' | ']') {
                        break;
                    }
                    key.push(next);
                    chars.next();
                }

                if key == "*" {
                    segments.push(Segment::Wildcard);
                } else if !key.is_empty() {
                    segments.push(Segment::Key(key));
                }
            }
            '[' => {
                let mut token = String::new();

                while let Some(&next) = chars.peek() {
                    if next == ']' {
                        break;
                    }
                    token.push(next);
                    chars.next();
                }

                if chars.next() != Some(']') {
                    return Err(Error::msg(format!("Invalid JSON path {path:?}")));
                }

                if token == "*" {
                    segments.push(Segment::Wildcard);
                } else {
                    let index = token
                        .parse::<usize>()
                        .map_err(|_| Error::msg(format!("Invalid JSON path {path:?}")))?;
                    segments.push(Segment::Index(index));
                }
            }
            ']' => return Err(Error::msg(format!("Invalid JSON path {path:?}"))),
            _ => {
                let mut key = String::from(character);

                while let Some(&next) = chars.peek() {
                    if matches!(next, '.' | '[' | ']') {
                        break;
                    }
                    key.push(next);
                    chars.next();
                }

                if key == "*" {
                    segments.push(Segment::Wildcard);
                } else {
                    segments.push(Segment::Key(key));
                }
            }
        }
    }

    let mut current = vec![root];

    for segment in segments {
        let mut next = vec![];

        match segment {
            Segment::Key(key) => {
                for value in current {
                    next.push(
                        value.get(&key).ok_or_else(|| {
                            Error::msg(format!("JSON path {path:?} did not match"))
                        })?,
                    );
                }
            }
            Segment::Index(index) => {
                for value in current {
                    next.push(
                        value
                            .as_array()
                            .and_then(|array| array.get(index))
                            .ok_or_else(|| {
                                Error::msg(format!("JSON path {path:?} did not match"))
                            })?,
                    );
                }
            }
            Segment::Wildcard => {
                for value in current {
                    if let Some(array) = value.as_array() {
                        next.extend(array);
                    } else if let Some(object) = value.as_object() {
                        next.extend(object.values());
                    } else {
                        return Err(Error::msg(format!("JSON path {path:?} did not match")));
                    }
                }
            }
        }

        if next.is_empty() {
            return Err(Error::msg(format!("JSON path {path:?} did not match")));
        }

        current = next;
    }

    Ok(current)
}
