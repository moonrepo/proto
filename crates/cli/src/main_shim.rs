// NOTE: We want to keep the shim binary as lean as possible,
// so these imports primarily use std, and avoid fat crates.

use anyhow::{anyhow, Result};
use proto_shim::{exec_command_and_replace, locate_proto_exe};
use rust_json::{json_parse, JsonElem as Json};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

fn get_proto_home() -> Result<PathBuf> {
    if let Ok(root) = env::var("PROTO_HOME") {
        return Ok(root.into());
    }

    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow!("Unable to determine user home directory."))?;

    Ok(home_dir.join(".proto"))
}

fn create_command(args: Vec<OsString>, shim_name: &str) -> Result<Command> {
    let proto_home_dir = get_proto_home()?;
    let registry_path = proto_home_dir.join("shims").join("registry.json");
    let mut shim = Json::Object(HashMap::default());

    // Load the shims registry if it exists
    if registry_path.exists() {
        let file = fs::read_to_string(registry_path)?;
        let mut registry = json_parse(&file).unwrap_or(Json::Null);

        if let Json::Object(shims) = &mut registry {
            if let Some(shim_entry) = shims.remove(shim_name) {
                if shim_entry.is_object() {
                    shim = shim_entry;
                }
            }
        }
    }

    // Determine args to pass to the underlying binary
    let mut passthrough_args = vec![];

    if let Json::Array(before_args) = &shim["before_args"] {
        for arg in before_args {
            if let Json::Str(arg) = arg {
                passthrough_args.push(OsString::from(arg));
            }
        }
    }

    if args.len() > 1 {
        for (i, arg) in args.into_iter().enumerate() {
            if i == 0 {
                continue; // The exe
            }

            passthrough_args.push(arg);
        }
    }

    if let Json::Array(after_args) = &shim["after_args"] {
        for arg in after_args {
            if let Json::Str(arg) = arg {
                passthrough_args.push(OsString::from(arg));
            }
        }
    }

    // Create the command and handle alternate logic
    let mut command = Command::new(locate_proto_exe("proto").unwrap_or_else(|| "proto".into()));

    // command.args(["run", "node", "--"]);
    // command.arg("./docs/shim-test.mjs");
    // command.arg("--version");

    if let Json::Str(parent_name) = &shim["parent"] {
        command.args(["run", parent_name]);

        if matches!(shim["alt_bin"], Json::Bool(true)) {
            command.args(["--alt", shim_name]);
        }
    } else {
        command.args(["run", shim_name]);
    }

    if !passthrough_args.is_empty() {
        command.arg("--");
        command.args(passthrough_args);
    }

    if let Json::Object(env_vars) = &shim["env_vars"] {
        for (env, value) in env_vars {
            if let Json::Str(var) = value {
                command.env(env, var);
            }
        }
    }

    Ok(command)
}

pub fn main() -> Result<()> {
    sigpipe::reset();

    // Extract arguments to pass-through
    let args = env::args_os().collect::<Vec<_>>();
    let exe_path = env::current_exe().unwrap_or_else(|_| PathBuf::from(&args[0]));

    // Extract the tool from the shim's file name
    let shim_name = exe_path
        .file_name()
        .map(|file| file.to_string_lossy())
        .unwrap_or_default()
        .replace(".exe", "");

    if shim_name.is_empty() || shim_name.contains("proto-shim") {
        return Err(anyhow!(
            "Invalid shim name detected. Unable to execute the appropriate proto tool.\nPlease refer to the documentation or ask for support on Discord."
        ));
    }

    // Create and execute the command
    let mut command = create_command(args, &shim_name)?;
    command.env("PROTO_SHIM_NAME", shim_name);
    command.env("PROTO_SHIM_PATH", exe_path);

    // Must be the last line!
    Ok(exec_command_and_replace(command)?)
}
