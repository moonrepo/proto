//! Host functions that are executed on the host machine, and are exposed
//! to WASM plugins (the guest) through the PDKs.

use crate::clients::{HttpClient, WarpgateHttpClientError};
use crate::plugin_error::WarpgatePluginError;
use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use starbase_console::EmptyReporter;
use starbase_process::{Command, Env};
use starbase_shell::ShellType;
use starbase_styles::{apply_style_tags, color};
use starbase_utils::net::{self, DownloadOptions};
use starbase_utils::{envx, fs};
use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use system_env::find_command_on_path;
use tokio::runtime::Handle;
use tracing::{debug, error, instrument, trace, warn};
use warpgate_api::{
    DownloadFileInput, DownloadFileOutput, ExecCommandInput, ExecCommandOutput, HostLogInput,
    HostLogTarget, SendRequestInput, SendRequestMethod, SendRequestOutput,
    convert_to_real_native_path,
};

/// Data passed to each host function.
#[derive(Clone)]
pub struct HostData {
    /// Location where cached files are stored.
    pub cache_dir: PathBuf,

    /// Instance of our HTTP client, used for sending requests.
    pub http_client: Arc<HttpClient>,

    /// Mapping of virtual paths, from host to guest paths.
    pub virtual_paths: Vec<(PathBuf, PathBuf)>,

    /// Current working directory, in which commands are executed.
    pub working_dir: PathBuf,
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
    ]
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
    let mut builder = Command::<EmptyReporter>::new(exe);
    builder.args(&input.args);

    match &shell_name {
        Some(shell_name) => {
            builder.set_shell(ShellType::from_str(shell_name)?);
        }
        None => {
            builder.no_shell();
        }
    };

    builder.cwd(&cwd);

    for (key, value) in &input.env {
        if let Some(key) = key.strip_suffix('?') {
            builder.env_with_behavior(key, Env::SetIfMissing(value.into()));
        } else if let Some(key) = key.strip_suffix('!') {
            builder.env_remove(key);
        } else {
            builder.env(key, value);
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

        // Set `PATH` explicitly instead of through `prepend_paths`, so that
        // it continues to take precedence over an inherited `PATH`
        builder.env("PATH", env::join_paths(paths)?);
    }

    // This host function is synchronous, and cannot await the crate's own
    // execution methods, so build the command and spawn it ourselves
    let mut command = builder
        .create_sync_command()
        .map_err(|error| Error::msg(error.to_string()))?;

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

    if name == "PATH" {
        let data = user_data.get()?;
        let data = data.lock().unwrap();

        // The WASM plugin has no context into what OS they are really
        // running on, so handle both delimiters for convenience.
        let new_path = value
            .replace(';', ":")
            .split(':')
            .map(|path| convert_to_real_native_path(path, &data.virtual_paths))
            .collect::<Vec<_>>();

        trace!(
            plugin = &uuid,
            name = &name,
            path = ?new_path,
            "Called host function {}",
            color::label("set_env_var"),
        );

        let mut path = envx::paths();
        path.extend(new_path);

        unsafe { env::set_var("PATH", env::join_paths(path)?) };
    } else {
        trace!(
            plugin = &uuid,
            name = &name,
            value = &value,
            "Called host function {}",
            color::label("set_env_var"),
        );

        unsafe { env::set_var(name, value) };
    }

    Ok(())
}
