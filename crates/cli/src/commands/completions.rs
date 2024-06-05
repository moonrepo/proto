use crate::app::App;
use crate::session::ProtoSession;
use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};
use clap_complete_nushell::Nushell;
use starbase::AppResult;
use starbase_shell::ShellType;
use std::process;

#[derive(Args, Clone, Debug)]
pub struct CompletionsArgs {
    #[arg(long, help = "Shell to generate for")]
    shell: Option<ShellType>,
}

#[tracing::instrument(skip_all)]
pub async fn completions(_session: ProtoSession, args: CompletionsArgs) -> AppResult {
    let shell = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    let mut app = App::command();
    let mut stdio = std::io::stdout();

    let clap_shell = match shell {
        ShellType::Bash => Shell::Bash,
        ShellType::Elvish => Shell::Elvish,
        ShellType::Fish => Shell::Fish,
        ShellType::Pwsh => Shell::PowerShell,
        ShellType::Zsh => Shell::Zsh,
        ShellType::Nu => {
            generate(Nushell, &mut app, "proto", &mut stdio);

            return Ok(());
        }
        unsupported => {
            eprintln!("{unsupported} does not currently support completions");

            process::exit(1);
        }
    };

    generate(clap_shell, &mut app, "proto", &mut stdio);

    Ok(())
}
