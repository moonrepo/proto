use shared_child::SharedChild;
use starbase::diagnostics::{self, miette, IntoDiagnostic, Result};
use starbase::tracing::{self, trace, TracingOptions};
use std::collections::VecDeque;
use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::{env, io, process};

pub fn main() -> Result<()> {
    sigpipe::reset();
    diagnostics::setup_miette();

    // Setup tracing and pass log level down
    let log_level = env::var("PROTO_LOG").unwrap_or_else(|_| "info".into());

    tracing::setup_tracing(TracingOptions {
        filter_modules: vec!["proto".into()],
        intercept_log: env::var("PROTO_WASM_LOG").is_err(),
        log_env: "PROTO_LOG".into(),
        ..TracingOptions::default()
    });

    // Extract arguments to pass-through
    let mut args = env::args().collect::<VecDeque<_>>();
    let exe_path = env::current_exe().unwrap_or_else(|_| PathBuf::from(&args[0]));

    let shim_name = exe_path
        .file_name()
        .map(|file| String::from_utf8_lossy(file.as_encoded_bytes()))
        .unwrap_or_default();

    trace!(args = ?args, shim = ?exe_path, "Running {} shim", shim_name);

    if shim_name.is_empty() || shim_name.contains("proto-shim") {
        return Err(miette!(
            code = "proto::invalid_shim",
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
            stdin.read_to_string(&mut buffer).into_diagnostic()?;
        }

        buffer
    };
    let has_piped_stdin = !input.is_empty();

    // Create the actual command to execute
    let mut command = Command::new(if cfg!(windows) { "proto.exe" } else { "proto" });
    command.args(["run", &shim_name]);

    if args.len() > 1 {
        args.pop_front();
        command.arg("--");
        command.args(args);
    }

    command.env("PROTO_LOG", log_level);

    if has_piped_stdin {
        command.stdin(Stdio::piped());
    }

    // Spawn a shareable child process
    trace!("Spawning proto child process");

    let shared_child = SharedChild::spawn(&mut command).into_diagnostic()?;
    let child = Arc::new(shared_child);
    let child_clone = Arc::clone(&child);

    // Handle CTRL+C and kill the child
    ctrlc::set_handler(move || {
        trace!("Received CTRL+C, killing child process");
        let _ = child_clone.kill();
    })
    .into_diagnostic()?;

    // If we have piped data, pass it through
    if has_piped_stdin {
        if let Some(mut stdin) = child.take_stdin() {
            trace!(input, "Received piped input, passing through");

            stdin.write_all(input.as_bytes()).into_diagnostic()?;
        }
    }

    // Wait for the process to finish or be killed
    let status = child.wait().into_diagnostic()?;
    let code = status.code().unwrap_or(0);

    trace!(code, "Received exit code");

    process::exit(code);
}
