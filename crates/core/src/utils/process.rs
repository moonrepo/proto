use miette::{Diagnostic, IntoDiagnostic};
use rustc_hash::FxHashMap;
use starbase_styles::{color, Style, Stylize};
use std::io;
use std::path::PathBuf;
use std::process::{Output, Stdio};
use thiserror::Error;
use tokio::process::Command;
use tracing::trace;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoProcessError {
    #[diagnostic(code(proto::process::command_failed))]
    #[error("Failed to execute command {}.", .command.style(Style::Shell))]
    FailedCommand {
        command: String,
        #[source]
        error: Box<io::Error>,
    },

    #[diagnostic(code(proto::process::command_failed))]
    #[error("Command {} returned a {code} exit code.", .command.style(Style::Shell))]
    FailedCommandNonZeroExit { command: String, code: i32 },
}

#[allow(dead_code)]
pub struct ProcessResult {
    pub command: String,
    pub exit_code: i32,
    pub stderr: String,
    pub stdout: String,
    pub working_dir: Option<PathBuf>,
}

async fn spawn_command(command: &mut Command) -> std::io::Result<Output> {
    let child = command.spawn()?;
    let output = child.wait_with_output().await?;

    Ok(output)
}

pub async fn exec_command(command: &mut Command) -> miette::Result<ProcessResult> {
    let inner = command.as_std();
    let command_line = format!(
        "{} {}",
        inner.get_program().to_string_lossy(),
        shell_words::join(
            inner
                .get_args()
                .map(|arg| arg.to_string_lossy())
                .collect::<Vec<_>>()
        )
    );

    trace!(
        cwd = ?inner.get_current_dir(),
        env = ?inner.get_envs()
            .filter_map(|(key, val)| val.map(|v| (key, v.to_string_lossy())))
            .collect::<FxHashMap<_, _>>(),
        "Running command {}", color::shell(&command_line)
    );

    let working_dir = inner.get_current_dir().map(PathBuf::from);
    let output =
        spawn_command(command)
            .await
            .map_err(|error| ProtoProcessError::FailedCommand {
                command: command_line.clone(),
                error: Box::new(error),
            })?;

    let stderr = String::from_utf8(output.stderr).into_diagnostic()?;
    let stdout = String::from_utf8(output.stdout).into_diagnostic()?;
    let code = output.status.code().unwrap_or(-1);

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
        stderr,
        stdout,
        exit_code: code,
        working_dir,
    })
}

pub async fn exec_command_piped(command: &mut Command) -> miette::Result<ProcessResult> {
    exec_command(command.stderr(Stdio::piped()).stdout(Stdio::piped())).await
}

pub async fn exec_command_with_privileges(
    command: &mut Command,
    elevated_program: Option<&str>,
) -> miette::Result<ProcessResult> {
    match elevated_program {
        Some(program) => {
            let inner = command.as_std();

            let mut sudo_command = Command::new(program);
            sudo_command.arg(inner.get_program());
            sudo_command.args(inner.get_args());

            for (key, value) in inner.get_envs() {
                if let Some(value) = value {
                    sudo_command.env(key, value);
                } else {
                    sudo_command.env_remove(key);
                }
            }

            if let Some(dir) = inner.get_current_dir() {
                sudo_command.current_dir(dir);
            }

            exec_command(&mut sudo_command).await
        }
        None => exec_command(command).await,
    }
}

pub async fn exec_command_with_privileges_piped(
    command: &mut Command,
    elevated_program: Option<&str>,
) -> miette::Result<ProcessResult> {
    exec_command_with_privileges(
        command.stderr(Stdio::piped()).stdout(Stdio::piped()),
        elevated_program,
    )
    .await
}
