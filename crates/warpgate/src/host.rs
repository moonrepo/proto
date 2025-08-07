use crate::client::HttpClient;
use crate::client_error::WarpgateClientError;
use crate::helpers;
use crate::plugin_error::WarpgatePluginError;
use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use starbase_shell::{ShellType, join_args};
use starbase_styles::{apply_style_tags, color};
use starbase_utils::env::paths;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use system_env::find_command_on_path;
use tokio::runtime::Handle;
use tracing::{debug, error, instrument, trace, warn};
use warpgate_api::{
    ExecCommandInput, ExecCommandOutput, HostLogInput, HostLogTarget, SendRequestInput,
    SendRequestOutput,
};

/// Data passed to each host function.
#[derive(Clone, Default)]
pub struct HostData {
    pub cache_dir: PathBuf,
    pub http_client: Arc<HttpClient>,
    pub virtual_paths: BTreeMap<PathBuf, PathBuf>,
    pub working_dir: PathBuf,
}

/// Create a list of our built-in host functions.
pub fn create_host_functions(data: HostData) -> Vec<Function> {
    vec![
        Function::new(
            "exec_command",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            exec_command,
        ),
        Function::new(
            "from_virtual_path",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            from_virtual_path,
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
        Function::new(
            "to_virtual_path",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            to_virtual_path,
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

fn get_default_shell() -> ShellType {
    static SHELL_CACHE: OnceLock<ShellType> = OnceLock::new();

    *SHELL_CACHE.get_or_init(ShellType::detect_with_fallback)
}

fn get_shell_exe_path(name: &str) -> PathBuf {
    // pwsh.exe isn't available on all Windows machines by default,
    // but powershell.exe typically is!
    if name == "pwsh" {
        return find_command_on_path("pwsh")
            .or_else(|| find_command_on_path("powershell"))
            .unwrap_or_else(|| "powershell".into());
    }

    find_command_on_path(name).unwrap_or_else(|| name.into())
}

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
        let path = helpers::from_virtual_path(&data.virtual_paths, PathBuf::from(&input.command));

        if path.exists() {
            // This is temporary since WASI does not support updating file permissions yet!
            if input.set_executable {
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
    let cwd = if let Some(cwd) = &input.cwd {
        helpers::from_virtual_path(&data.virtual_paths, cwd)
    } else {
        data.working_dir.clone()
    };

    // Determine the shell
    let shell = match input.shell.or_else(|| env::var("PROTO_SHELL").ok()) {
        Some(name) => ShellType::from_str(&name)?.build(),
        None => get_default_shell().build(),
    };
    let shell_command = shell.get_exec_command();
    let shell_exe_name = shell.to_string();
    let shell_exe_path = get_shell_exe_path(&shell_exe_name);

    // Create and execute command
    let command_line = format!("{} {}", input.command, join_args(&shell, &input.args));

    let mut command = Command::new(&shell_exe_path);
    command.args(shell_command.shell_args);
    command.envs(&input.env);
    command.current_dir(&cwd);

    if shell_command.pass_args_stdin {
        command.stdin(Stdio::piped());
    } else {
        command.arg(&command_line);
        command.stdin(Stdio::null());
    }

    if should_stream {
        command.stderr(Stdio::inherit()).stdout(Stdio::inherit());
    } else {
        command.stderr(Stdio::piped()).stdout(Stdio::piped());
    }

    let mut child = command.spawn()?;
    let pid = child.id();

    if shell_command.pass_args_stdin {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(command_line.as_bytes())?;
        }
    }

    trace!(
        plugin = &uuid,
        shell = &shell_exe_name,
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
        }
    } else {
        let result = child.wait_with_output()?;

        ExecCommandOutput {
            command: input.command.clone(),
            exit_code: result.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
        }
    };

    trace!(
        plugin = plugin.id().to_string(),
        shell = ?shell_exe_path,
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
        url = &input.url,
        "Sending request from host machine"
    );

    let response = Handle::current().block_on(async {
        let mut client = data.http_client.get(&input.url);

        for (name, value) in input.headers {
            client = client.header(name, value);
        }

        if let Some(timeout) = plugin.time_remaining() {
            client = client.timeout(timeout);
        }

        client
            .send()
            .await
            .map_err(|error| HttpClient::map_error(input.url.clone(), error))
    })?;

    let ok = response.status().is_success();
    let status = response.status().as_u16();

    let bytes = Handle::current().block_on(async {
        response
            .bytes()
            .await
            .map_err(|error| WarpgateClientError::Http {
                url: input.url.clone(),
                error: Box::new(error),
            })
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
            .map(|path| helpers::from_virtual_path(&data.virtual_paths, PathBuf::from(path)))
            .collect::<Vec<_>>();

        trace!(
            plugin = &uuid,
            name = &name,
            path = ?new_path,
            "Called host function {}",
            color::label("set_env_var"),
        );

        let mut path = paths();
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

#[instrument(name = "host_func_from_virtual_path", skip_all)]
fn from_virtual_path(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostData>,
) -> Result<(), Error> {
    let original_path = PathBuf::from(plugin.memory_get_val::<String>(&inputs[0])?);
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        original_path = ?original_path,
        "Calling host function {}",
        color::label("from_virtual_path"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let real_path = helpers::from_virtual_path(&data.virtual_paths, &original_path);

    trace!(
        plugin = &uuid,
        real_path = ?real_path,
        "Called host function {}",
        color::label("from_virtual_path"),
    );

    plugin.memory_set_val(&mut outputs[0], real_path.to_string_lossy().to_string())?;

    Ok(())
}

#[instrument(name = "host_func_to_virtual_path", skip_all)]
fn to_virtual_path(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostData>,
) -> Result<(), Error> {
    let original_path = PathBuf::from(plugin.memory_get_val::<String>(&inputs[0])?);
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        original_path = ?original_path,
        "Calling host function {}",
        color::label("to_virtual_path"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let virtual_path = helpers::to_virtual_path(&data.virtual_paths, &original_path);

    trace!(
        plugin = &uuid,
        virtual_path = ?virtual_path.virtual_path(),
        "Called host function {}",
        color::label("to_virtual_path"),
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&virtual_path)?)?;

    Ok(())
}
