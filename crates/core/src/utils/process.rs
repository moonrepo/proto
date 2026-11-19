use crate::reporter::ProtoReporter;
use starbase_process::{Arg, Command, ProcessError, StdinMode, output_to_string};
use starbase_styles::{Style, Stylize, color};
use starbase_utils::fs::FsError;
use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{Level, enabled, trace};

/// A child process command, built and executed with `starbase_process`.
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
/// shell, like the standard library does.
pub fn new_command<T: AsRef<OsStr>>(bin: T) -> ProtoCommand {
    let mut command = ProtoCommand::new(bin);
    command.no_shell();
    configure_command(&mut command);
    command
}

/// Create a command that executes `bin` through the detected shell, which
/// quotes the arguments and resolves the executable, for example, a script
/// that may not be directly executable on its own.
pub fn new_shell_command<T: AsRef<OsStr>>(bin: T) -> ProtoCommand {
    let mut command = ProtoCommand::new(bin);
    configure_command(&mut command);
    command
}

fn configure_command(command: &mut ProtoCommand) {
    // We inspect the exit code ourselves (see `handle_exec`), so a non-zero
    // exit must come back as a result, not an error
    command.set_error_on_nonzero(false);

    // Children have always inherited stdin, so that package managers, `sudo`,
    // and others can prompt. They also aren't contained (the default), so
    // they stay in our process group, where the terminal signals them directly
    command.set_stdin(StdinMode::Inherit);

    // Only log explicitly set environment variables when tracing, as they
    // may contain secrets
    command.debug.print_env = enabled!(Level::TRACE);
}

async fn exec(
    command: &mut ProtoCommand,
    capture: bool,
) -> Result<ProcessResult, ProtoProcessError> {
    let command_line = command.get_command_line(false, false);
    let working_dir = command.cwd.as_ref().map(PathBuf::from);

    // `starbase_process` logs the command line, environment, and duration
    let output = if capture {
        command.exec_capture_output().await
    } else {
        command.exec_stream_output().await
    }
    .map_err(|error| ProtoProcessError::FailedCommand {
        command: command_line.clone(),
        error: Box::new(extract_io_error(error)),
    })?;

    let code = output.code().unwrap_or(-1);
    let stderr = output_to_string(&output.stderr);
    let stdout = output_to_string(&output.stdout);

    trace!(
        code,
        stderr = if stderr.len() > 250 {
            "<truncated>"
        } else {
            &stderr
        },
        stdout = if stdout.len() > 250 {
            "<truncated>"
        } else {
            &stdout
        },
        "Ran command {}",
        color::shell(&command_line)
    );

    Ok(ProcessResult {
        command: command_line,
        exit_code: code,
        stderr,
        stdout,
        working_dir,
    })
}

// Every failure that can surface here (non-zero exits are not errors) wraps
// the underlying IO error, so unwrap it to keep our error's shape
pub(crate) fn extract_io_error(report: miette::Report) -> io::Error {
    match report.downcast::<ProcessError>() {
        Ok(
            ProcessError::Capture { error, .. }
            | ProcessError::Stream { error, .. }
            | ProcessError::StreamCapture { error, .. }
            | ProcessError::WriteInput { error, .. },
        ) => *error,
        Ok(error) => io::Error::other(error.to_string()),
        Err(report) => io::Error::other(report.to_string()),
    }
}

/// Execute the command, inheriting stdout and stderr, so that its output
/// streams straight to the terminal, and nothing is captured.
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

// Run the elevated program in place of the executable, which becomes its
// first argument, so that the environment, working directory, shell, and
// other arguments carry over as-is
fn elevate_command(command: &mut ProtoCommand, elevated_program: Option<&str>) {
    let Some(program) = elevated_program else {
        return;
    };

    let exe = Arg::from(command.exe.as_os_str());

    command.args.push_front(exe);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevates_the_executable_into_the_first_arg() {
        let mut command = new_command("apt-get");
        command.args(["install", "-y", "git"]).env("KEY", "value");

        elevate_command(&mut command, Some("sudo"));

        assert_eq!(command.get_bin_name(), "sudo");
        assert_eq!(command.get_args_list(), ["apt-get", "install", "-y", "git"]);
        assert!(command.contains_env("KEY"));
        assert!(command.shell.is_none());
    }

    #[test]
    fn doesnt_elevate_without_a_program() {
        let mut command = new_command("apt-get");
        command.arg("update");

        elevate_command(&mut command, None);

        assert_eq!(command.get_bin_name(), "apt-get");
        assert_eq!(command.get_args_list(), ["update"]);
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn captures_output_and_nonzero_exit_codes() {
        let mut command = new_shell_command("echo");
        command.args(["a  b", "&&", "exit", "3"]);

        // Args are quoted for the shell when needed, but operators pass through
        let result = exec_command_piped(&mut command).await.unwrap();

        assert_eq!(result.exit_code, 3);
        assert_eq!(result.stdout.trim(), "a  b");

        let mut command = ProtoCommand::new_script("echo out && echo err >&2 && exit 3");
        configure_command(&mut command);

        let result = exec_command_piped(&mut command).await.unwrap();

        assert_eq!(result.exit_code, 3);
        assert_eq!(result.stdout.trim(), "out");
        assert_eq!(result.stderr.trim(), "err");
        assert!(handle_exec(result).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn errors_for_a_missing_executable() {
        let mut command = new_command("proto-missing-executable-for-testing");

        let error = exec_command_piped(&mut command).await.err().unwrap();

        assert!(matches!(
            error,
            ProtoProcessError::FailedCommand { error, .. } if error.kind() == io::ErrorKind::NotFound
        ));
    }
}
