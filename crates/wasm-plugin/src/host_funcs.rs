use extism::{CurrentPlugin, Error, Function, InternalExt, UserData, Val, ValType};
use proto_pdk_api::{ExecCommandInput, ExecCommandOutput, HostLogInput, PluginError};
use std::path::PathBuf;
use std::process::Command;
use tracing::trace;

#[derive(Debug)]
pub struct HostData {
    pub working_dir: PathBuf,
}

pub fn create_host_functions(data: HostData) -> Vec<Function> {
    vec![
        Function::new("host_log", [ValType::I64], [], None, host_log),
        Function::new(
            "exec_command",
            [ValType::I64],
            [ValType::I64],
            Some(UserData::new(data)),
            exec_command,
        ),
        // Backwards compatibility
        Function::new("trace", [ValType::I64], [], None, host_log),
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

    // let data = user_data.any().unwrap();
    // let data = data.downcast_ref::<HostData>().unwrap();

    let mut command = Command::new(&input.command);
    command.args(&input.args);
    command.envs(&input.env_vars);
    // command.current_dir(&data.working_dir)

    let output = if input.stream {
        let result = command.spawn()?.wait()?;

        ExecCommandOutput {
            exit_code: result.code().unwrap_or(0),
            stderr: String::new(),
            stdout: String::new(),
        }
    } else {
        let result = command.output()?;

        ExecCommandOutput {
            exit_code: result.status.code().unwrap_or(0),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
        }
    };

    trace!(
        target: "proto_wasm::exec_command",
        command = &input.command,
        exit_code = output.exit_code,
        stderr_len = output.stderr.len(),
        stdout_len = output.stdout.len(),
        "Executed command from plugin"
    );

    if output.exit_code != 0 {
        let mut command_line = vec![input.command];
        command_line.extend(input.args);

        return Err(PluginError::Message(format!(
            "Command `{}` failed with a {} exit code: {}",
            command_line.join(" "),
            output.exit_code,
            output.stderr
        ))
        .into());
    }

    let output_str = serde_json::to_string(&output)?;
    let ptr = plugin.memory_alloc_bytes(output_str)?;

    outputs[0] = Val::I64(ptr as i64);

    Ok(())
}
