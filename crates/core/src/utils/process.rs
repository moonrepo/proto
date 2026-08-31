use crate::reporter::ProtoReporter;
use starbase_process::{Arg, Command, ProcessError, output_to_string};
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// A command that renders through proto's console reporter.
pub type ProtoCommand = Command<ProtoReporter>;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoProcessError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[diagnostic(code(proto::process::command_failed))]
    #[error("Failed to execute command {}.", .command.style(Style::Shell))]
    FailedCommand {
        command: String,
        #[source]
        error: Box<io::Error>,
    },

    #[diagnostic(code(proto::process::command_failed))]
    #[error(
        "Command {} returned a {code} exit code.\n{}",
        .command.style(Style::Shell),
        .stderr.style(Style::MutedLight),
    )]
    FailedCommandNonZeroExit {
        command: String,
        code: i32,
        stderr: String,
    },
}

impl From<FsError> for ProtoProcessError {
    fn from(e: FsError) -> ProtoProcessError {
        ProtoProcessError::Fs(Box::new(e))
    }
}

#[allow(dead_code)]
pub struct ProcessResult {
    pub command: String,
    pub exit_code: i32,
    pub stderr: String,
    pub stdout: String,
    pub working_dir: Option<PathBuf>,
}

/// Create a command that executes `bin` directly, without wrapping it in a
/// shell, mirroring how the standard library spawns a process.
pub fn new_command<T: AsRef<OsStr>>(bin: T) -> ProtoCommand {
    let mut command = ProtoCommand::new(bin);
    command.no_shell();
    configure_command(&mut command);
    command
}

/// Create a command that executes a full command line, which always requires
/// a shell, instead of a single executable.
pub fn new_script_command<T: AsRef<OsStr>>(script: T) -> ProtoCommand {
    let mut command = ProtoCommand::new_script(script);
    configure_command(&mut command);
    command
}

/// Create a command that executes `bin` through the detected shell, so that
/// the executable and its arguments are quoted and resolved by the shell.
pub fn new_shell_command<T: AsRef<OsStr>>(bin: T) -> ProtoCommand {
    let mut command = ProtoCommand::new(bin);
    configure_command(&mut command);
    command
}

fn configure_command(command: &mut ProtoCommand) {
    // proto inspects the exit code itself and decides what to do with it
    // through `handle_exec`, so a non-zero exit must come back as a result
    // and not as an error
    command.set_error_on_nonzero(false);

    // Avoid logging the entire environment, which is mostly noise and may
    // contain secrets inherited from the parent process
    command.debug.env_key_prefixes = vec!["PROTO_".into()];
}

async fn exec(
    command: &mut ProtoCommand,
    capture: bool,
) -> Result<ProcessResult, ProtoProcessError> {
    let command_line = command.get_command_line(false, false);
    let working_dir = command.cwd.as_ref().map(PathBuf::from);

    // `starbase_process` logs the command line, environment, and duration,
    // and registers the child so that it's shut down with proto
    let output = if capture {
        command.exec_capture_output().await
    } else {
        command.exec_stream_output().await
    }
    .map_err(|error| to_failed_command(command_line.clone(), error))?;

    Ok(ProcessResult {
        command: command_line,
        exit_code: output.code().unwrap_or(-1),
        stderr: output_to_string(&output.stderr),
        stdout: output_to_string(&output.stdout),
        working_dir,
    })
}

fn to_failed_command(command: String, report: miette::Report) -> ProtoProcessError {
    // Every failure that can surface here wraps the underlying IO error, so
    // unwrap it to keep the shape of our own error unchanged
    let error = match report.downcast::<ProcessError>() {
        Ok(
            ProcessError::Capture { error, .. }
            | ProcessError::Stream { error, .. }
            | ProcessError::StreamCapture { error, .. }
            | ProcessError::WriteInput { error, .. },
        ) => error,
        Ok(error) => Box::new(io::Error::other(error.to_string())),
        Err(report) => Box::new(io::Error::other(report.to_string())),
    };

    ProtoProcessError::FailedCommand { command, error }
}

/// Execute the command, inheriting stdout and stderr, so that its output
/// streams straight to the terminal and nothing is captured.
pub async fn exec_command(command: &mut ProtoCommand) -> Result<ProcessResult, ProtoProcessError> {
    exec(command, false).await
}

/// Execute the command, capturing stdout and stderr instead of streaming them.
pub async fn exec_command_piped(
    command: &mut ProtoCommand,
) -> Result<ProcessResult, ProtoProcessError> {
    exec(command, true).await
}

/// Execute the command through an elevated program, like `sudo`. See
/// [`exec_command`].
pub async fn exec_command_with_privileges(
    command: &mut ProtoCommand,
    elevated_program: Option<&str>,
) -> Result<ProcessResult, ProtoProcessError> {
    elevate_command(command, elevated_program);

    exec_command(command).await
}

/// Execute the command through an elevated program, like `sudo`. See
/// [`exec_command_piped`].
pub async fn exec_command_with_privileges_piped(
    command: &mut ProtoCommand,
    elevated_program: Option<&str>,
) -> Result<ProcessResult, ProtoProcessError> {
    elevate_command(command, elevated_program);

    exec_command_piped(command).await
}

// Push the executable down into the first argument and run the elevated
// program in its place, so that the environment, working directory, and
// remaining arguments carry over as-is. Only binaries are elevated, never
// scripts, so the executable is always a single program name.
fn elevate_command(command: &mut ProtoCommand, elevated_program: Option<&str>) {
    let Some(program) = elevated_program else {
        return;
    };

    let exe = command.exe.as_os_str().to_os_string();

    command.args.push_front(Arg::from(exe));
    command.set_bin(program);
}

pub fn handle_exec(result: ProcessResult) -> Result<ProcessResult, ProtoProcessError> {
    if result.exit_code != 0 {
        return Err(ProtoProcessError::FailedCommandNonZeroExit {
            command: result.command.clone(),
            code: result.exit_code,
            stderr: result.stderr.clone(),
        });
    }

    Ok(result)
}
