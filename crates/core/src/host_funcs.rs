use crate::proto::ProtoEnvironment;
use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use proto_pdk_api::{ExecCommandInput, ExecCommandOutput, HostLogInput, HostLogTarget};
use starbase_utils::fs;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use system_env::create_process_command;
use tracing::trace;
use warpgate::Id;

#[derive(Clone)]
pub struct HostData {
    pub id: Id,
    pub proto: Arc<ProtoEnvironment>,
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

pub fn host_log(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    _user_data: UserData<HostData>,
) -> Result<(), Error> {
    let input: HostLogInput = serde_json::from_str(plugin.memory_get_val(&inputs[0])?)?;

    match input {
        HostLogInput::Message(message) => {
            trace!(
                target: "proto_wasm::log",
                "{}", message,
            );
        }
        HostLogInput::TargetedMessage { message, target } => {
            match target {
                HostLogTarget::Stderr => {
                    eprintln!("{}", message);
                }
                HostLogTarget::Stdout => {
                    println!("{}", message);
                }
                HostLogTarget::Tracing => {
                    trace!(
                        target: "proto_wasm::log",
                        "{}", message,
                    );
                }
            };
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
    user_data: UserData<HostData>,
) -> Result<(), Error> {
    let input: ExecCommandInput = serde_json::from_str(plugin.memory_get_val(&inputs[0])?)?;

    trace!(
        target: "proto_wasm::exec_command",
        command = &input.command,
        args = ?input.args,
        env_vars = ?input.env_vars,
        "Executing command from plugin"
    );

    // This is temporary since WASI does not support updating file permissions yet!
    if input.set_executable && PathBuf::from(&input.command).exists() {
        fs::update_perms(&input.command, None)?;
    }

    let data = user_data.get()?;
    let data = data.lock().unwrap();

    let mut command = create_process_command(&input.command, &input.args);
    command.envs(&input.env_vars);
    command.current_dir(&data.proto.cwd);

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

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&output)?)?;

    Ok(())
}

fn get_env_var(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    _user_data: UserData<HostData>,
) -> Result<(), Error> {
    let name: String = plugin.memory_get_val(&inputs[0])?;
    let value = env::var(&name).unwrap_or_default();

    trace!(
        target: "proto_wasm::get_env_var",
        name = &name,
        value = &value,
        "Read environment variable from host"
    );

    plugin.memory_set_val(&mut outputs[0], value)?;

    Ok(())
}

fn set_env_var(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    _user_data: UserData<HostData>,
) -> Result<(), Error> {
    let name: String = plugin.memory_get_val(&inputs[0])?;
    let value: String = plugin.memory_get_val(&inputs[1])?;

    trace!(
        target: "proto_wasm::set_env_var",
        name = &name,
        value = &value,
        "Wrote environment variable to host"
    );

    env::set_var(name, value);

    Ok(())
}

fn from_virtual_path(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostData>,
) -> Result<(), Error> {
    let virtual_path = PathBuf::from(plugin.memory_get_val::<String>(&inputs[0])?);

    let data = user_data.get()?;
    let data = data.lock().unwrap();

    let paths_map = data.proto.get_virtual_paths();
    let real_path = warpgate::from_virtual_path(&paths_map, &virtual_path);

    trace!(
        target: "proto_wasm::from_virtual_path",
        virtual_path = ?virtual_path,
        real_path = ?real_path,
        "Converted a virtual path into a real path"
    );

    plugin.memory_set_val(&mut outputs[0], real_path.to_str().unwrap())?;

    Ok(())
}

fn to_virtual_path(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostData>,
) -> Result<(), Error> {
    let real_path = PathBuf::from(plugin.memory_get_val::<String>(&inputs[0])?);

    let data = user_data.get()?;
    let data = data.lock().unwrap();

    let paths_map = data.proto.get_virtual_paths();
    let virtual_path = warpgate::to_virtual_path(&paths_map, &real_path);

    trace!(
        target: "proto_wasm::to_virtual_path",
        real_path = ?real_path,
        virtual_path = ?virtual_path,
        "Converted a real path into a virtual path"
    );

    plugin.memory_set_val(&mut outputs[0], virtual_path.to_str().unwrap())?;

    Ok(())
}
