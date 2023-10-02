use extism::{CurrentPlugin, Error, Function, InternalExt, UserData, Val, ValType};
use proto_pdk_api::{ExecCommandInput, ExecCommandOutput, HostLogInput};
use starbase_utils::fs;
use std::env;
use std::path::PathBuf;
use std::process::Command;
use tracing::trace;

#[derive(Debug)]
pub struct HostData {
    pub working_dir: PathBuf,
}

pub fn create_host_functions(data: HostData) -> Vec<Function> {
    vec![
        Function::new(
            "exec_command",
            [ValType::I64],
            [ValType::I64],
            Some(UserData::new(data)),
            exec_command,
        ),
        Function::new(
            "get_env_var",
            [ValType::I64],
            [ValType::I64],
            None,
            get_env_var,
        ),
        Function::new("host_log", [ValType::I64], [], None, host_log),
        Function::new(
            "set_env_var",
            [ValType::I64, ValType::I64],
            [],
            None,
            set_env_var,
        ),
    ]
}

// Logging

pub fn host_log(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    _user_data: UserData,
) -> Result<(), Error> {
    let input_str = plugin.memory_read_str(inputs[0].unwrap_i64() as u64)?;
    let input: HostLogInput = serde_json::from_str(input_str)?;

    match input {
        HostLogInput::Message(message) => {
            trace!(
                target: "proto_wasm::log",
                "{}", message,
            );
        }
        HostLogInput::Fields { data, message } => {
            trace!(
                target: "proto_wasm::log",
                data = ?data,
                "{}", message,
            );
        }
    };

    Ok(())
}

// Commands

fn exec_command(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    _user_data: UserData,
) -> Result<(), Error> {
    let input_str = plugin.memory_read_str(inputs[0].unwrap_i64() as u64)?;
    let input: ExecCommandInput = serde_json::from_str(input_str)?;

    trace!(
        target: "proto_wasm::exec_command",
        command = &input.command,
        args = ?input.args,
        env_vars = ?input.env_vars,
        "Executing command from plugin"
    );

    // This is temporary since WASI does not support updating file permissions yet!
    if input.set_executable {
        fs::update_perms(&input.command, None)?;
    }

    // let data = user_data.any().unwrap();
    // let data = data.downcast_ref::<HostData>().unwrap();

    let mut command = Command::new(&input.command);
    command.args(&input.args);
    command.envs(&input.env_vars);
    // command.current_dir(&data.working_dir)

    let output = if input.stream {
        let result = command.spawn()?.wait()?;

        ExecCommandOutput {
            command: input.command.clone(),
            exit_code: result.code().unwrap_or(0),
            stderr: String::new(),
            stdout: String::new(),
        }
    } else {
        let result = command.output()?;

        ExecCommandOutput {
            command: input.command.clone(),
            exit_code: result.status.code().unwrap_or(0),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
        }
    };

    let debug_output = env::var("PROTO_DEBUG_COMMAND").is_ok_and(|v| !v.is_empty());

    trace!(
        target: "proto_wasm::exec_command",
        command = &input.command,
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

    let output_str = serde_json::to_string(&output)?;
    let ptr = plugin.memory_alloc_bytes(output_str)?;

    outputs[0] = Val::I64(ptr as i64);

    Ok(())
}

fn get_env_var(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    _user_data: UserData,
) -> Result<(), Error> {
    let name = plugin.memory_read_str(inputs[0].unwrap_i64() as u64)?;
    let value = env::var(name).unwrap_or_default();

    trace!(
        target: "proto_wasm::get_env_var",
        name,
        value = &value,
        "Reading environment variable from host"
    );

    let ptr = plugin.memory_alloc_bytes(value)?;

    outputs[0] = Val::I64(ptr as i64);

    Ok(())
}

fn set_env_var(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    _user_data: UserData,
) -> Result<(), Error> {
    let name = plugin
        .memory_read_str(inputs[0].unwrap_i64() as u64)?
        .to_owned();

    let value = plugin
        .memory_read_str(inputs[1].unwrap_i64() as u64)?
        .to_owned();

    trace!(
        target: "proto_wasm::set_env_var",
        name = &name,
        value = &value,
        "Writing environment variable to host"
    );

    env::set_var(name, value);

    Ok(())
}
