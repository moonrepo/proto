use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use proto_pdk_api::{ExecCommandInput, ExecCommandOutput, TraceInput};
use std::path::PathBuf;
use std::process::Command;
use tracing::trace;

#[derive(Debug)]
pub struct HostData {
    pub working_dir: PathBuf,
}

pub fn create_functions(data: HostData) -> Vec<Function> {
    vec![
        Function::new("trace", [ValType::I64], [], None, log_trace),
        Function::new(
            "exec_command",
            [ValType::I64],
            [ValType::I64],
            Some(UserData::new(data)),
            exec_command,
        ),
    ]
}

// Logging

pub fn log_trace(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    _user_data: UserData,
) -> Result<(), Error> {
    let input_str = unsafe { (*plugin.memory).get_str(inputs[0].unwrap_i64() as usize)? };
    let input: TraceInput = serde_json::from_str(input_str)?;

    match input {
        TraceInput::Message(message) => {
            trace!(
                target: "proto::wasm::trace",
                "{}", message,
            );
        }
        TraceInput::Fields { data, message } => {
            trace!(
                target: "proto::wasm::trace",
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
    let input_str = unsafe { (*plugin.memory).get_str(inputs[0].unwrap_i64() as usize)? };
    let input: ExecCommandInput = serde_json::from_str(input_str)?;

    trace!(
        target: "proto::wasm::exec_command",
        command = &input.command,
        args = ?input.args,
        env_vars = ?input.env_vars,
        "Executing command",
    );

    // let data = user_data.any().unwrap();
    // let data = data.downcast_ref::<HostData>().unwrap();

    let result = Command::new(&input.command)
        .args(input.args)
        .envs(input.env_vars)
        // .current_dir(&data.working_dir)
        .output()?;

    let output = ExecCommandOutput {
        exit_code: result.status.code().unwrap_or(0),
        stderr: String::from_utf8_lossy(&result.stderr).to_string(),
        stdout: String::from_utf8_lossy(&result.stdout).to_string(),
    };

    trace!(
        target: "proto::wasm::exec_command",
        command = &input.command,
        exit_code = output.exit_code,
        stderr_len = output.stderr.len(),
        stdout_len = output.stdout.len(),
        "Executed command"
    );

    let output_str = serde_json::to_string(&output)?;
    let ptr = unsafe { (*plugin.memory).alloc_bytes(output_str)? };

    outputs[0] = Val::I64(ptr.offset as i64);

    Ok(())
}
