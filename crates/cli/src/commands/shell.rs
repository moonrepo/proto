use super::exec::*;
use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use clap::Args;
use starbase::AppResult;
use starbase_shell::ShellType;

#[derive(Args, Clone, Debug)]
pub struct ShellArgs {
    #[arg(help = "Tools to initialize")]
    tools: Vec<String>,

    #[arg(long, help = "Shell to start a session for")]
    shell: Option<ShellType>,
}

#[tracing::instrument(skip_all)]
pub async fn shell(session: ProtoSession, args: ShellArgs) -> AppResult {
    // Detect the shell that we need to activate for
    let shell_type = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    // Define the interactive command to use
    let command = match shell_type {
        ShellType::Ash => "ash -i",
        ShellType::Bash => "bash -i",
        ShellType::Elvish => "elvish",
        ShellType::Fish => "fish --interactive",
        ShellType::Ion => "ion",
        ShellType::Murex => "murex",
        ShellType::Nu => "nu --interactive",
        ShellType::Pwsh => "pwsh -Interactive -NoLogo",
        ShellType::Sh => "sh",
        ShellType::Xonsh => "xonsh --interactive",
        ShellType::Zsh => "zsh --interactive",
        ShellType::PowerShell => {
            return Err(ProtoCliError::ShellPowerShellNotSupported.into());
        }
    };

    // Passthrough to exec
    exec(
        session,
        ExecArgs {
            tools_from_config: args.tools.is_empty(),
            tools: args.tools,
            raw: false,
            shell: None,
            command: command
                .split_whitespace()
                .map(|arg| arg.to_owned())
                .collect(),
        },
    )
    .await
}
