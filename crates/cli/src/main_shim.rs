// NOTE: We want to keep the shim binary as lean as possible,
// so these imports use std as much as possible, and should
// not pull in large libraries (tracing is already enough)!

use anyhow::{anyhow, Result};
use rust_json::{json_parse, JsonElem as Json};
use shared_child::SharedChild;
use starbase::tracing::{self, trace, TracingOptions};
use std::collections::HashMap;
use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::{env, fs, io, process};

fn get_proto_home() -> Result<PathBuf> {
    if let Ok(root) = env::var("PROTO_HOME") {
        return Ok(root.into());
    }

    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow!("Unable to determine user home directory."))?;

    Ok(home_dir.join(".proto"))
}

fn create_command(args: Vec<String>, shim_name: &str) -> Result<Command> {
    let registry_path = get_proto_home()?.join("shims/registry.json");
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
                passthrough_args.push(arg);
            }
        }
    }

    if args.len() > 1 {
        for (i, arg) in args.iter().enumerate() {
            if i == 0 {
                continue; // The exe
            }

            passthrough_args.push(arg);
        }
    }

    if let Json::Array(after_args) = &shim["after_args"] {
        for arg in after_args {
            if let Json::Str(arg) = arg {
                passthrough_args.push(arg);
            }
        }
    }

    // Create a command for local testing
    // let mut command = Command::new("node");
    // command.arg("./docs/shim-test.mjs");

    // Create the command and handle alternate logic
    let mut command = Command::new(if cfg!(windows) { "proto.exe" } else { "proto" });

    if let Json::Str(alt_for) = &shim["alt_for"] {
        command.args(["run", alt_for, "--alt", shim_name]);
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

    // Setup tracing and pass log level down
    let log_level = env::var("PROTO_LOG").unwrap_or_else(|_| "info".into());

    tracing::setup_tracing(TracingOptions {
        filter_modules: vec!["proto".into()],
        intercept_log: env::var("PROTO_WASM_LOG").is_err(),
        log_env: "PROTO_LOG".into(),
        ..TracingOptions::default()
    });

    // Extract arguments to pass-through
    let args = env::args().collect::<Vec<_>>();
    let exe_path = env::current_exe().unwrap_or_else(|_| PathBuf::from(&args[0]));

    let shim_name = exe_path
        .file_name()
        .map(|file| String::from_utf8_lossy(file.as_encoded_bytes()))
        .unwrap_or_default()
        .replace(".exe", "");

    trace!(args = ?args, shim = ?exe_path, "Running {} shim", shim_name);

    if shim_name.is_empty() || shim_name.contains("proto-shim") {
        return Err(anyhow!(
            "Invalid shim name detected. Unable to execute the appropriate proto tool.\nPlease refer to the documentation or ask for support on Discord."
        ));
    }

    // Capture any piped input
    let input = {
        let mut stdin = io::stdin();
        let mut buffer = String::new();

        // Only read piped data when stdin is not a TTY,
        // otherwise the process will hang indefinitely waiting for EOF
        if !stdin.is_terminal() {
            stdin.read_to_string(&mut buffer)?;
        }

        buffer
    };
    let has_piped_stdin = !input.is_empty();

    // Create the actual command to execute
    let mut command = create_command(args, &shim_name)?;
    command.env("PROTO_LOG", log_level);

    if has_piped_stdin {
        command.stdin(Stdio::piped());
    }

    // Spawn a shareable child process
    trace!("Spawning proto child process");

    let shared_child = SharedChild::spawn(&mut command)?;
    let child = Arc::new(shared_child);
    let child_clone = Arc::clone(&child);

    // Handle CTRL+C and kill the child
    ctrlc::set_handler(move || {
        trace!("Received CTRL+C, killing child process");
        let _ = child_clone.kill();
    })?;

    // If we have piped data, pass it through
    if has_piped_stdin {
        if let Some(mut stdin) = child.take_stdin() {
            trace!(input, "Received piped input, passing through");
            stdin.write_all(input.as_bytes())?;
        }
    }

    // Wait for the process to finish or be killed
    let status = child.wait()?;
    let code = status.code().unwrap_or(0);

    trace!(code, "Received exit code");

    process::exit(code);
}
