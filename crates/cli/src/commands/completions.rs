use crate::app::App;
use crate::session::ProtoSession;
use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};
use clap_complete_nushell::Nushell;
use iocraft::prelude::element;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_shell::ShellType;

#[derive(Args, Clone, Debug)]
pub struct CompletionsArgs {
    #[arg(long, help = "Shell to generate for")]
    shell: Option<ShellType>,
}

#[tracing::instrument(skip_all)]
pub async fn completions(session: ProtoSession, args: CompletionsArgs) -> AppResult {
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

            return Ok(None);
        }
        unsupported => {
            session.console.render(element! {
                Notice(variant: Variant::Caution) {
                    StyledText(
                        content: format!(
                            "<id>{unsupported}</id> does not currently support completions",
                        ),
                    )
                }
            })?;

            return Ok(Some(1));
        }
    };

    generate(clap_shell, &mut app, "proto", &mut stdio);

    Ok(None)
}
