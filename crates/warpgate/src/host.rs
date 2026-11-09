//! Host functions that are executed on the host machine, and are exposed
//! to WASM plugins (the guest) through the PDKs.

use crate::clients::{HttpClient, WarpgateHttpClientError};
use crate::plugin_error::WarpgatePluginError;
use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use rustc_hash::FxHashMap;
use regex::Regex;
use starbase_shell::{ShellType, join_exe_args};
use starbase_styles::{apply_style_tags, color};
use starbase_utils::net::{self, DownloadOptions};
use starbase_utils::{envx, fs};
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use system_env::find_command_on_path;
use tokio::runtime::Handle;
use tracing::{debug, error, instrument, trace, warn};
use warpgate_api::{
    DownloadFileInput, DownloadFileOutput, ExecCommandInput, ExecCommandOutput, HostLogInput,
    HostLogTarget, LoadToolVersionsInput, LoadToolVersionsOutput, SendRequestInput,
    SendRequestMethod, SendRequestOutput, convert_to_real_native_path,
};

/// Data passed to each host function.
#[derive(Clone)]
pub struct HostData {
    /// Location where cached files are stored.
    pub cache_dir: PathBuf,

    /// The platform the plugin is targeting, when it's not the host machine.
    /// Executing a command logs a warning while set, as its output only
    /// reflects the host, and may be wrong for the targeted platform.
    pub foreign_platform: Option<String>,

    /// Instance of our HTTP client, used for sending requests.
    pub http_client: Arc<HttpClient>,

    /// Mapping of virtual paths, from host to guest paths.
    pub virtual_paths: Vec<(PathBuf, PathBuf)>,

    /// Current working directory, in which commands are executed.
    pub working_dir: PathBuf,

    /// Optional per-tool overrides for the version source.
    pub versions_source: Option<ToolVersionsSource>,
}

/// Configured version source overrides for a tool.
#[derive(Clone, Debug)]
pub struct ToolVersionsSource {
    pub version_pattern: Option<String>,
    pub index_url: Option<String>,
    pub index_version_key: Option<String>,
    pub git_url: Option<String>,
    pub git_tag_pattern: Option<String>,
}

/// Create a list of our built-in host functions.
pub fn create_host_functions(data: HostData) -> Vec<Function> {
    vec![
        Function::new(
            "download_file",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            download_file,
        ),
        Function::new(
            "exec_command",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            exec_command,
        ),
        Function::new(
            "get_env_var",
            [ValType::I64],
            [ValType::I64],
            UserData::new(()),
            get_env_var,
        ),
        Function::new(
            "get_env_vars",
            [ValType::I64],
            [ValType::I64],
            UserData::new(()),
            get_env_vars,
        ),
        Function::new(
            "load_tool_versions",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            load_tool_versions,
        ),
        Function::new("host_log", [ValType::I64], [], UserData::new(()), host_log),
        Function::new(
            "send_request",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            send_request,
        ),
        Function::new(
            "set_env_var",
            [ValType::I64, ValType::I64],
            [],
            UserData::new(data.clone()),
            set_env_var,
        ),
        Function::new(
            "set_env_vars",
            [ValType::I64],
            [],
            UserData::new(data.clone()),
            set_env_vars,
        ),
    ]
}

#[instrument(name = "host_func_load_tool_versions", skip_all)]
fn load_tool_versions(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostData>,
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
                load_index_source(&data, source, index_url, None, plugin.time_remaining())?
            } else {
                load_git_source(
                    &data,
                    source,
                    git_url.unwrap_or(&url),
                    tag_pattern.as_deref(),
                )?
            }
        }
        LoadToolVersionsInput::Index { url, version_path } => {
            if let Some(git_url) = git_url {
                load_git_source(&data, source, git_url, None)?
            } else {
                load_index_source(
                    &data,
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

    let versions = load_tool_versions_from_git(data, url)?;

    filter_versions(versions, pattern, "Git tag")
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

// Logging

#[instrument(name = "host_func_log", skip_all)]
fn host_log(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    _user_data: UserData<()>,
) -> Result<(), Error> {
    let input: HostLogInput = serde_json::from_str(plugin.memory_get_val(&inputs[0])?)?;
    let message = apply_style_tags(input.message);

    match input.target {
        HostLogTarget::Stderr => {
            if input.data.is_empty() {
                eprintln!("{message}");
            } else {
                eprintln!(
                    "{message} {}",
                    color::muted_light(format!("({:?})", input.data)),
                );
            }
        }
        HostLogTarget::Stdout => {
            if input.data.is_empty() {
                println!("{message}");
            } else {
                println!(
                    "{message} {}",
                    color::muted_light(format!("({:?})", input.data)),
                );
            }
        }
        // Levels
        HostLogTarget::Debug => {
            debug!(data = ?input.data, "{message}");
        }
        HostLogTarget::Error => {
            error!(data = ?input.data, "{message}");
        }
        HostLogTarget::Warn => {
            warn!(data = ?input.data, "{message}");
        }
        _ => {
            trace!(data = ?input.data, "{message}");
        }
    };

    Ok(())
}

// Commands

#[instrument(name = "host_func_exec_command", skip_all)]
fn exec_command(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostData>,
) -> Result<(), Error> {
    let instant = Instant::now();
    let input_raw: String = plugin.memory_get_val(&inputs[0])?;
    let input: ExecCommandInput = serde_json::from_str(&input_raw)?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        input = %input_raw,
        "Calling host function {}",
        color::label("exec_command"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();

    // Plugins can check `HostEnvironment.foreign` themselves, but older
    // plugins don't know about it, so call out a potentially wrong result
    if let Some(platform) = &data.foreign_platform {
        warn!(
            plugin = &uuid,
            "Plugin executed command {} while targeting {platform}, but the output reflects the host machine, not {platform}",
            color::shell(&input.command),
        );
    }

    let debug_output = env::var("WARPGATE_DEBUG_COMMAND").ok();
    let should_stream = input.stream
        || debug_output
            .as_ref()
            .is_some_and(|level| level == "all" || level == "stream");

    // Relative or absolute file path
    let maybe_exe = if input.command.contains('/') || input.command.contains('\\') {
        let path = convert_to_real_native_path(&input.command, &data.virtual_paths);

        if path.exists() {
            // This is temporary since WASI does not support updating file permissions yet!
            if input.set_executable && !fs::is_executable(&path) {
                fs::update_perms(&path, None)?;
            }

            Some(path)
        } else {
            None
        }
    }
    // Command on PATH
    else {
        find_command_on_path(&input.command)
    };

    let Some(exe) = &maybe_exe else {
        return Err(WarpgatePluginError::MissingCommand {
            command: input.command.clone(),
        }
        .into());
    };

    // Determine working directory
    let cwd = input
        .cwd
        .as_ref()
        .map(|cwd| convert_to_real_native_path(cwd, &data.virtual_paths))
        .unwrap_or_else(|| data.working_dir.clone());

    // Determine the shell
    let shell_name = input.shell.or_else(|| env::var("PROTO_SHELL").ok());

    // Create and execute command
    let mut command = match &shell_name {
        Some(shell_name) => {
            let shell = ShellType::from_str(shell_name)?.build();
            shell.create_wrapped_command_with(join_exe_args(&shell, exe, &input.args, false))
        }
        None => {
            let mut command = Command::new(exe);
            command.args(&input.args);
            command
        }
    };

    command.current_dir(&cwd);

    for (key, value) in &input.env {
        if let Some(key) = key.strip_suffix('?') {
            if env::var_os(key).is_none() {
                command.env(key, value);
            }
        } else if let Some(key) = key.strip_suffix('!') {
            command.env_remove(key);
        } else {
            command.env(key, value);
        }
    }

    if !input.paths.is_empty() {
        let env_paths = envx::paths();
        let mut paths = Vec::with_capacity(env_paths.len() + input.paths.len());

        paths.extend(
            input
                .paths
                .iter()
                .map(|path| convert_to_real_native_path(path, &data.virtual_paths)),
        );
        paths.extend(env_paths);

        command.env("PATH", env::join_paths(paths)?);
    }

    command.stdin(Stdio::null());

    if should_stream {
        command.stderr(Stdio::inherit()).stdout(Stdio::inherit());
    } else {
        command.stderr(Stdio::piped()).stdout(Stdio::piped());
    }

    let mut child = command.spawn()?;
    let pid = child.id();

    trace!(
        plugin = &uuid,
        shell = &shell_name,
        exe = &input.command,
        args = ?input.args,
        cwd = ?cwd,
        pid = pid,
        "Executing command on host machine"
    );

    let output = if should_stream {
        let result = child.wait()?;

        ExecCommandOutput {
            command: input.command.clone(),
            exit_code: result.code().unwrap_or(-1),
            stderr: String::new(),
            stdout: String::new(),
            streamed: true,
        }
    } else {
        let result = child.wait_with_output()?;

        ExecCommandOutput {
            command: input.command.clone(),
            exit_code: result.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
            streamed: false,
        }
    };

    trace!(
        plugin = plugin.id().to_string(),
        shell = &shell_name,
        exe = ?exe,
        pid = pid,
        exit_code = output.exit_code,
        stderr = if debug_output.is_some() {
            Some(&output.stderr)
        } else {
            None
        },
        stderr_len = output.stderr.len(),
        stdout = if debug_output.is_some() {
            Some(&output.stdout)
        } else {
            None
        },
        stdout_len = output.stdout.len(),
        "Called host function {} in {:?}",
        color::label("exec_command"),
        instant.elapsed()
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&output)?)?;

    Ok(())
}

#[instrument(name = "host_func_send_request", skip_all)]
fn send_request(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostData>,
) -> Result<(), Error> {
    let instant = Instant::now();
    let input_raw: String = plugin.memory_get_val(&inputs[0])?;
    let input: SendRequestInput = serde_json::from_str(&input_raw)?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        input = %input_raw,
        "Calling host function {}",
        color::label("send_request"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();

    trace!(
        plugin = &uuid,
        method = ?input.method,
        url = &input.url,
        "Sending request from host machine"
    );

    let (ok, status, bytes) = Handle::current().block_on(async {
        let mut client = match input.method {
            SendRequestMethod::Get => data.http_client.get(&input.url),
            SendRequestMethod::Post => data.http_client.post(&input.url),
        };

        for (name, value) in input.headers {
            client = client.header(name, data.http_client.expand_env_vars(&value));
        }

        if let Some(timeout) = plugin.time_remaining() {
            client = client.timeout(timeout);
        }

        let response = client
            .send()
            .await
            .map_err(|error| HttpClient::map_error(input.url.clone(), error))?;

        let ok = response.status().is_success();
        let status = response.status().as_u16();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| WarpgateHttpClientError::Http {
                url: input.url.clone(),
                error: Box::new(error),
            })?;

        Ok::<_, WarpgateHttpClientError>((ok, status, bytes))
    })?;

    // Create and return our intermediate shapes
    let memory = plugin.memory_new(Vec::from(bytes))?;

    let output = SendRequestOutput {
        body: Vec::new(),
        body_length: memory.length,
        body_offset: memory.offset,
        status,
    };

    trace!(
        plugin = &uuid,
        ok,
        status,
        length = memory.length,
        "Called host function {} in {:?}",
        color::label("send_request"),
        instant.elapsed()
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&output)?)?;

    Ok(())
}

#[instrument(name = "host_func_download_file", skip_all)]
fn download_file(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostData>,
) -> Result<(), Error> {
    let instant = Instant::now();
    let input_raw: String = plugin.memory_get_val(&inputs[0])?;
    let input: DownloadFileInput = serde_json::from_str(&input_raw)?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        input = %input_raw,
        "Calling host function {}",
        color::label("download_file"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();

    let dest_file = convert_to_real_native_path(&input.file, &data.virtual_paths);

    trace!(
        plugin = &uuid,
        url = &input.url,
        file = ?dest_file,
        "Downloading file on host machine"
    );

    Handle::current().block_on(net::download_from_url_with_options(
        &input.url,
        &dest_file,
        DownloadOptions {
            downloader: Some(Box::new(
                data.http_client
                    .create_downloader_with_headers(input.headers),
            )),
            ..Default::default()
        },
    ))?;

    let output = DownloadFileOutput {
        file: input.file,
        size: fs::metadata(&dest_file)?.len(),
    };

    trace!(
        plugin = &uuid,
        size = output.size,
        "Called host function {} in {:?}",
        color::label("download_file"),
        instant.elapsed()
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&output)?)?;

    Ok(())
}

#[instrument(name = "host_func_get_env_var", skip_all)]
fn get_env_var(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    _user_data: UserData<()>,
) -> Result<(), Error> {
    let name: String = plugin.memory_get_val(&inputs[0])?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        name = &name,
        "Calling host function {}",
        color::label("get_env_var"),
    );

    let value = env::var(&name).unwrap_or_default();

    trace!(
        plugin = &uuid,
        value = &value,
        "Called host function {}",
        color::label("get_env_var"),
    );

    plugin.memory_set_val(&mut outputs[0], value)?;

    Ok(())
}

#[instrument(name = "host_func_get_env_vars", skip_all)]
fn get_env_vars(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    _user_data: UserData<()>,
) -> Result<(), Error> {
    let names: Vec<String> = serde_json::from_str(plugin.memory_get_val(&inputs[0])?)?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        names = ?names,
        "Calling host function {}",
        color::label("get_env_vars"),
    );

    // Unlike `get_env_var`, variables that are not set (or are not
    // valid Unicode) are omitted, so the guest can tell them apart
    // from variables that are set to an empty value
    let vars = names
        .into_iter()
        .filter_map(|name| env::var(&name).ok().map(|value| (name, value)))
        .collect::<FxHashMap<_, _>>();

    trace!(
        plugin = &uuid,
        vars = ?vars,
        "Called host function {}",
        color::label("get_env_vars"),
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&vars)?)?;

    Ok(())
}

#[instrument(name = "host_func_set_env_var", skip_all)]
fn set_env_var(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    user_data: UserData<HostData>,
) -> Result<(), Error> {
    let name: String = plugin.memory_get_val(&inputs[0])?;
    let value: String = plugin.memory_get_val(&inputs[1])?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        name = &name,
        value = &value,
        "Calling host function {}",
        color::label("set_env_var"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();

    set_host_env_var(&uuid, "set_env_var", &data, name, value)
}

#[instrument(name = "host_func_set_env_vars", skip_all)]
fn set_env_vars(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    user_data: UserData<HostData>,
) -> Result<(), Error> {
    let vars: FxHashMap<String, String> = serde_json::from_str(plugin.memory_get_val(&inputs[0])?)?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        vars = ?vars,
        "Calling host function {}",
        color::label("set_env_vars"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();

    for (name, value) in vars {
        set_host_env_var(&uuid, "set_env_vars", &data, name, value)?;
    }

    Ok(())
}

// Shared by `set_env_var` and `set_env_vars`. Setting `PATH` appends
// to the current value instead of overwriting it.
fn set_host_env_var(
    uuid: &str,
    func: &str,
    data: &HostData,
    name: String,
    value: String,
) -> Result<(), Error> {
    if name == "PATH" {
        // The WASM plugin has no context into what OS they are really
        // running on, so handle both delimiters for convenience.
        let new_path = value
            .replace(';', ":")
            .split(':')
            .map(|path| convert_to_real_native_path(path, &data.virtual_paths))
            .collect::<Vec<_>>();

        trace!(
            plugin = uuid,
            name = &name,
            path = ?new_path,
            "Called host function {}",
            color::label(func),
        );

        let mut path = envx::paths();
        path.extend(new_path);

        unsafe { env::set_var("PATH", env::join_paths(path)?) };
    } else {
        trace!(
            plugin = uuid,
            name = &name,
            value = &value,
            "Called host function {}",
            color::label(func),
        );

        unsafe { env::set_var(name, value) };
    }

    Ok(())
}
