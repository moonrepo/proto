use crate::error::WarpgateError;
use crate::helpers;
use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use starbase_styles::color::{self, apply_style_tags};
use starbase_utils::env::{bool_var, paths};
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use system_env::{create_process_command, find_command_on_path};
use tokio::runtime::Handle;
use tracing::{instrument, trace};
use warpgate_api::{
    ExecCommandInput, ExecCommandOutput, HostLogInput, HostLogTarget, SendRequestInput,
    SendRequestOutput,
};

#[derive(Clone)]
pub struct HostData {
    pub http_client: Arc<reqwest::Client>,
    pub virtual_paths: BTreeMap<PathBuf, PathBuf>,
    pub working_dir: PathBuf,
}

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
            UserData::new(data.clone()),
            get_env_var,
        ),
        Function::new(
            "host_log",
            [ValType::I64],
            [],
            UserData::new(data.clone()),
            host_log,
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
    _user_data: UserData<HostData>,
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
        HostLogTarget::Tracing => {
            if input.data.is_empty() {
                trace!("{message}");
            } else {
                trace!(
                    data = ?input.data,
                    "{message}"
                );
            }
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
    let input: ExecCommandInput = serde_json::from_str(plugin.memory_get_val(&inputs[0])?)?;

    let data = user_data.get()?;
    let data = data.lock().unwrap();

    // Relative or absolute file path
    let maybe_bin = if input.command.contains('/') || input.command.contains('\\') {
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
    // Command on PATH
    } else {
        find_command_on_path(&input.command)
    };

    let Some(bin) = &maybe_bin else {
        return Err(WarpgateError::PluginCommandMissing {
            command: input.command.clone(),
        }
        .into());
    };

    // Determine working directory
    let cwd = if let Some(working_dir) = &input.working_dir {
        helpers::from_virtual_path(&data.virtual_paths, working_dir)
    } else {
        data.working_dir.clone()
    };

    trace!(
        command = &input.command,
        args = ?input.args,
        env = ?input.env,
        cwd = ?cwd,
        "Executing command from plugin"
    );

    let mut command = create_process_command(bin, &input.args);
    command.envs(&input.env);
    command.current_dir(cwd);

    let output = if input.stream {
        let result = command.spawn()?.wait()?;

        ExecCommandOutput {
            command: input.command.clone(),
            exit_code: result.code().unwrap_or(-1),
            stderr: String::new(),
            stdout: String::new(),
        }
    } else {
        let result = command.output()?;

        ExecCommandOutput {
            command: input.command.clone(),
            exit_code: result.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
        }
    };

    let debug_output = bool_var("WARPGATE_DEBUG_COMMAND");

    trace!(
        command = ?bin,
        exit_code = output.exit_code,
        stderr = if debug_output {
            Some(&output.stderr)
        } else {
            None
        },
        stderr_len = output.stderr.len(),
        stdout = if debug_output {
            Some(&output.stdout)
        } else {
            None
        },
        stdout_len = output.stdout.len(),
        "Executed command from plugin"
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
    let input: SendRequestInput = serde_json::from_str(plugin.memory_get_val(&inputs[0])?)?;

    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let handle = Handle::current();

    trace!(url = &input.url, "Sending request from plugin");

    let response = handle.block_on(async {
        let mut client = data.http_client.get(&input.url);

        if let Some(timeout) = plugin.time_remaining() {
            client = client.timeout(timeout);
        }

        client.send().await.map_err(|error| WarpgateError::Http {
            url: input.url.clone(),
            error: Box::new(error),
        })
    })?;

    let status = response.status().as_u16();

    trace!(
        url = &input.url,
        length = response.content_length(),
        status,
        ok = response.status().is_success(),
        "Sent request from plugin"
    );

    let body = handle.block_on(async {
        response.bytes().await.map_err(|error| WarpgateError::Http {
            url: input.url.clone(),
            error: Box::new(error),
        })
    })?;

    let memory = plugin.memory_new(Vec::from(body))?;

    let output = SendRequestOutput {
        body: Vec::new(),
        body_length: memory.length,
        body_offset: memory.offset,
        status,
    };

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&output)?)?;

    Ok(())
}

#[instrument(name = "host_func_get_env_var", skip_all)]
fn get_env_var(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    _user_data: UserData<HostData>,
) -> Result<(), Error> {
    let name: String = plugin.memory_get_val(&inputs[0])?;
    let value = env::var(&name).unwrap_or_default();

    trace!(
        name = &name,
        value = &value,
        "Read environment variable from host"
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
            name = &name,
            path = ?new_path,
            "Adding paths to PATH environment variable on host"
        );

        let mut path = paths();
        path.extend(new_path);

        env::set_var("PATH", env::join_paths(path)?);
    } else {
        trace!(
            name = &name,
            value = &value,
            "Wrote environment variable to host"
        );

        env::set_var(name, value);
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

    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let real_path = helpers::from_virtual_path(&data.virtual_paths, &original_path);

    trace!(
        original_path = ?original_path,
        real_path = ?real_path,
        "Converted a path into a real path"
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

    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let virtual_path = helpers::to_virtual_path(&data.virtual_paths, &original_path);

    trace!(
        original_path = ?original_path,
        virtual_path = ?virtual_path.virtual_path(),
        "Converted a path into a virtual path"
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&virtual_path)?)?;

    Ok(())
}
