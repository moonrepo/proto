use crate::session::ProtoSession;
use crate::shell::{
    Export, find_first_profile, format_exports, prompt_for_shell, prompt_for_shell_profile,
    update_profile_if_not_setup,
};
use clap::Args;
use iocraft::FlexDirection;
use iocraft::prelude::{View, element};
use proto_shim::get_exe_file_name;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_shell::{BoxedShell, ShellType};
use starbase_styles::color;
use std::env;
use std::path::PathBuf;
use tracing::debug;

#[cfg(windows)]
mod windows;

#[derive(Args, Clone, Debug)]
pub struct SetupArgs {
    #[arg(long, help = "Shell to setup for")]
    shell: Option<ShellType>,

    #[arg(
        long,
        help = "Don't update a shell profile",
        alias = "no-profile",
        env = "PROTO_NO_MODIFY_PROFILE"
    )]
    no_modify_profile: bool,

    #[arg(
        long,
        help = "Don't update the system path",
        env = "PROTO_NO_MODIFY_PATH"
    )]
    no_modify_path: bool,
}

const DISCORD: &str = "https://discord.gg/qCh9MEynv2";

#[tracing::instrument(skip_all)]
pub async fn setup(session: ProtoSession, args: SetupArgs) -> AppResult {
    let paths = starbase_utils::env::paths();

    if paths.contains(&session.env.store.shims_dir) || paths.contains(&session.env.store.bin_dir) {
        debug!("Skipping setup, proto already exists in PATH");

        session.console.render(element! {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: format!(
                        "Successfully installed proto to <path>{}</path>!",
                        session.env.store.bin_dir.join(get_exe_file_name("proto")).display()
                    ),
                )
                StyledText(
                    content: format!(
                        "Need help? Join our Discord <url>{}</url>",
                        DISCORD
                    ),
                    style: Style::MutedLight
                )
            }
        })?;

        return Ok(None);
    }

    debug!("Determining the shell to use");

    let interactive = !session.should_skip_prompts();

    let shell_type = match args.shell.or_else(ShellType::detect) {
        Some(value) => value,
        None => {
            if interactive {
                debug!("Unable to detect, prompting the user to select a shell");

                prompt_for_shell(&session.console).await?
            } else {
                ShellType::default()
            }
        }
    };

    let shell = shell_type.build();
    let exported_content = format_exports(
        &shell,
        "proto",
        vec![
            Export::Var(
                "PROTO_HOME".into(),
                env::var("PROTO_HOME").unwrap_or_else(|_| {
                    if env::var("XDG_DATA_HOME").is_ok() {
                        "$XDG_DATA_HOME/proto"
                    } else {
                        "$HOME/.proto"
                    }
                    .into()
                }),
            ),
            Export::Path(vec!["$PROTO_HOME/shims".into(), "$PROTO_HOME/bin".into()]),
        ],
    );

    let modified_profile_path = if args.no_modify_profile {
        None
    } else {
        update_shell_profile(&session, &shell, &exported_content, interactive).await?
    };

    #[allow(clippy::needless_bool)]
    let modified_system_env_path = if args.no_modify_path {
        false
    } else {
        #[cfg(windows)]
        {
            windows::do_add_to_path(vec![
                session.env.store.shims_dir.clone(),
                session.env.store.bin_dir.clone(),
            ])?
        }

        #[cfg(unix)]
        true
    };

    let bin_path = session.env.store.bin_dir.join(get_exe_file_name("proto"));
    let should_launch_terminal = modified_system_env_path || modified_profile_path.is_some();
    let should_print_exports = !exported_content.is_empty() && modified_profile_path.is_none();

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            #(if let Some(shell_path) = modified_profile_path {
                element! {
                    Stack {
                        StyledText(
                            content: format!(
                                "Successfully installed proto to <path>{}</path>,",
                                bin_path.display()
                            ),
                        )
                        StyledText(
                            content: format!(
                                "and updated the shell profile at <path>{}</path>!",
                                shell_path.display()
                            ),
                        )
                    }
                }.into_any()
            } else {
                element! {
                    StyledText(
                        content: format!(
                            "Successfully installed proto to <path>{}</path>!",
                            bin_path.display()
                        ),
                    )
                }.into_any()
            })

            #(if should_launch_terminal {
                element! {
                    View(margin_top: 1) {
                        StyledText(
                            content: "Launch a new terminal to start using proto!",
                            style: Style::Success
                        )
                    }
                }
            } else {
                element! {
                    View(margin_top: 1, flex_direction: FlexDirection::Column) {
                        #(if should_print_exports {
                            element! {
                                Stack {
                                    StyledText(
                                        content: "Add the following to your shell profile and launch a new terminal to get started:"
                                    )
                                    View(padding_top: 1, padding_left: 2) {
                                        StyledText(
                                            content: exported_content.trim(),
                                            style: Style::MutedLight
                                        )
                                    }
                                }
                            }
                        } else {
                            element! {
                                Stack {
                                    StyledText(
                                        content: "Add the following to your <property>PATH</property> to get started:",
                                    )
                                    View(padding_top: 1, padding_left: 2) {
                                        StyledText(
                                            content: if cfg!(windows) {
                                                format!(
                                                    "{};{}",
                                                    session.env.store.shims_dir.display(),
                                                    session.env.store.bin_dir.display()
                                                )
                                            } else {
                                                "$HOME/.proto/shims:$HOME/.proto/bin".into()
                                            },
                                            style: Style::MutedLight
                                        )
                                    }
                                }
                            }
                        })
                    }
                }
            })

            View(margin_top: 1) {
                StyledText(
                    content: format!(
                        "Need help? Join our Discord <url>{}</url>",
                        DISCORD
                    )
                )
            }
        }
    })?;

    Ok(None)
}

async fn update_shell_profile(
    session: &ProtoSession,
    shell: &BoxedShell,
    content: &str,
    interactive: bool,
) -> miette::Result<Option<PathBuf>> {
    debug!("Updating PATH in {} shell", shell);

    let profile_path = if interactive {
        debug!("Prompting the user to select a shell profile");

        prompt_for_shell_profile(&session.console, shell, &session.env.home_dir).await?
    } else {
        debug!("Attempting to find a shell profile to update");

        find_first_profile(shell, &session.env.home_dir).ok()
    };

    // If we found a profile, update the global config so we can reference it
    if let Some(profile) = &profile_path {
        debug!("Selected profile {}, updating", color::path(profile));

        update_profile_if_not_setup(profile, content, "PROTO_HOME")?;

        session.env.store.save_preferred_profile(profile)?;
    }

    Ok(profile_path)
}
