use crate::app::App;
use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};
use starbase::system;
use starbase_shell::ShellType;
use std::process;

#[derive(Args, Clone, Debug)]
pub struct CompletionsArgs {
    #[arg(long, help = "Shell to generate for")]
    shell: Option<ShellType>,
}

#[system]
pub async fn completions(args: ArgsRef<CompletionsArgs>) {
    let shell = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    let clap_shell = match shell {
        ShellType::Bash => Shell::Bash,
        ShellType::Elvish => Shell::Elvish,
        ShellType::Fish => Shell::Fish,
        ShellType::Pwsh => Shell::PowerShell,
        ShellType::Zsh => Shell::Zsh,
        unsupported => {
            eprintln!("{unsupported} does not currently support completions");

            process::exit(1);
        }
    };

    let mut app = App::command();
    let mut stdio = std::io::stdout();

    generate(clap_shell, &mut app, "proto", &mut stdio);
}
