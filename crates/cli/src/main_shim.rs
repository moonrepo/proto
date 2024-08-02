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

static mut DEBUG: Option<bool> = None;

// We don't want to pull the entire `tracing` or `log` crates
// into this binary, as we want it to be super lean. So we have
// this very rudimentary logging system.
fn debug(op: impl FnOnce() -> String) {
    unsafe {
        if DEBUG.is_none() {
            DEBUG = Some(env::var("PROTO_DEBUG_SHIM").is_ok());
        }

        if DEBUG.is_some_and(|enabled| enabled) {
            println!("{}", op());
        }
    }
}

fn get_proto_home() -> Result<PathBuf> {
    debug(|| "Determining proto home direcory".into());

    if let Ok(root) = env::var("PROTO_HOME") {
        debug(|| format!("Found in `PROTO_HOME` environment variable: {root}"));

        return Ok(root.into());
    }

    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow!("Unable to determine user home directory."))?
        .join(".proto");

    debug(|| format!("Using system home directory: {home_dir:?}"));

    Ok(home_dir)
}

fn create_command(args: Vec<OsString>, shim_name: &str) -> Result<Command> {
    let proto_home_dir = get_proto_home()?;
    let registry_path = proto_home_dir.join("shims").join("registry.json");
    let mut shim = Json::Object(HashMap::default());

    // Load the shims registry if it exists
    if registry_path.exists() {
        debug(|| format!("Loading shim registry config: {registry_path:?}"));

        let file = fs::read_to_string(registry_path)?;
        let mut registry = json_parse(&file).unwrap_or(Json::Null);

        debug(|| format!("Loaded: {file}"));
        debug(|| format!("Extracting {shim_name} config"));

        if let Json::Object(shims) = &mut registry {
            if let Some(shim_entry) = shims.remove(shim_name) {
                if shim_entry.is_object() {
                    shim = shim_entry;
                    debug(|| "Extracted".into());
                } else {
                    debug(|| "Not extracted, config is not an object".into());
                }
            } else {
                debug(|| "Not extracted, key does not exist".into());
            }
        }
    }

    // Determine args to pass to the underlying binary
    let mut passthrough_args = vec![];

    if let Json::Array(before_args) = &shim["before_args"] {
        debug(|| "Inheriting config `before_args`".into());

        for arg in before_args {
            if let Json::Str(arg) = arg {
                passthrough_args.push(OsString::from(arg));
            }
        }
    }

    if args.len() > 1 {
        debug(|| "Inheriting args passed on the command line".into());

        for (i, arg) in args.into_iter().enumerate() {
            if i == 0 {
                continue; // The exe
            }

            passthrough_args.push(arg);
        }
    }

    if let Json::Array(after_args) = &shim["after_args"] {
        debug(|| "Inheriting config `after_args`".into());

        for arg in after_args {
            if let Json::Str(arg) = arg {
                passthrough_args.push(OsString::from(arg));
            }
        }
    }

    // Create the command and handle alternate logic
    let proto_bin = locate_proto_exe("proto").unwrap_or_else(|| "proto".into());

    debug(|| format!("Locating proto binary: {proto_bin:?}"));

    let mut command = Command::new(proto_bin);

    // command.args(["run", "node", "--"]);
    // command.arg("./docs/shim-test.mjs");
    // command.arg("--version");

    if let Json::Str(parent_name) = &shim["parent"] {
        debug(|| "Inheriting config `parent`".into());
        debug(|| format!("Running parent tool {parent_name}"));

        command.args(["run", parent_name]);

        if matches!(shim["alt_bin"], Json::Bool(true)) {
            debug(|| "Inheriting config `alt_bin`".into());
            debug(|| format!("Running tool alternate {shim_name}"));

            command.args(["--alt", shim_name]);
        }
    } else {
        debug(|| format!("Running tool {shim_name}"));

        command.args(["run", shim_name]);
    }

    if !passthrough_args.is_empty() {
        debug(|| format!("Passing through arguments: {passthrough_args:?}"));

        command.arg("--");
        command.args(passthrough_args);
    }

    if let Json::Object(env_vars) = &shim["env_vars"] {
        debug(|| "Inheriting config `env_vars`".into());

        for (env, value) in env_vars {
            if let Json::Str(var) = value {
                command.env(env, var);
            }
        }
    }

    debug(|| "Created proto command".into());

    Ok(command)
}

pub fn main() -> Result<()> {
    sigpipe::reset();

    debug(|| "Running proto shim".into());

    // Extract arguments to pass-through
    let args = env::args_os().collect::<Vec<_>>();

    debug(|| format!("Extracting arguments: {args:?}"));

    let exe_path = env::current_exe().unwrap_or_else(|_| PathBuf::from(&args[0]));

    debug(|| format!("Extracting current executable: {exe_path:?}"));

    // Extract the tool from the shim's file name
    let shim_name = exe_path
        .file_name()
        .map(|file| file.to_string_lossy())
        .unwrap_or_default()
        .to_lowercase()
        .replace(".exe", "");

    debug(|| format!("Determining tool from shim name: {shim_name}"));

    if shim_name.is_empty() || shim_name.contains("proto-shim") {
        return Err(anyhow!(
            "Invalid shim name detected. Unable to execute the appropriate proto tool.\nPlease refer to the documentation or ask for support on Discord."
        ));
    }

    // Create and execute the command
    debug(|| "Creating proto command with arguments".into());

    let mut command = create_command(args, &shim_name)?;
    command.env("PROTO_SHIM_NAME", shim_name);
    command.env("PROTO_SHIM_PATH", exe_path);

    debug(|| "Executing proto command".into());
    debug(|| "This will replace the current process and stop debugging!".into());

    // Must be the last line!
    Ok(exec_command_and_replace(command)?)
}
