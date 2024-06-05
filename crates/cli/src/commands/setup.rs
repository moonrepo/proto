use crate::session::ProtoSession;
use crate::shell::{
    format_exports, prompt_for_shell, prompt_for_shell_profile, write_profile,
    write_profile_if_not_setup, Export,
};
use clap::Args;
use proto_shim::get_exe_file_name;
use starbase::AppResult;
use starbase_shell::{BoxedShell, ShellError, ShellType};
use starbase_styles::color;
use std::env;
use std::io::stdout;
use std::io::IsTerminal;
use std::path::PathBuf;
use tracing::debug;

#[cfg(windows)]
mod windows;

#[derive(Args, Clone, Debug)]
pub struct SetupArgs {
    #[arg(long, help = "Shell to setup for")]
    shell: Option<ShellType>,

    #[arg(long, help = "Don't update a shell profile", alias = "no-profile")]
    no_modify_profile: bool,

    #[arg(long, help = "Don't update the system path")]
    no_modify_path: bool,

    #[arg(long, short = 'y', help = "Avoid interactive prompts and use defaults")]
    yes: bool,
}

#[tracing::instrument(skip_all)]
pub async fn setup(session: ProtoSession, args: SetupArgs) -> AppResult {
    let paths = env::split_paths(&env::var("PATH").unwrap()).collect::<Vec<_>>();

    if paths.contains(&session.env.store.shims_dir) && paths.contains(&session.env.store.bin_dir) {
        debug!("Skipping setup, proto already exists in PATH");

        already_setup_message(&session);

        return Ok(());
    }

    debug!("Determining the shell to use");

    let interactive = !args.yes && env::var("CI").is_err() && stdout().is_terminal();

    let shell_type = match args.shell.or_else(ShellType::detect) {
        Some(value) => value,
        None => {
            if interactive {
                debug!("Unable to detect, prompting the user to select a shell");

                prompt_for_shell()?
            } else {
                return Err(ShellError::CouldNotDetectShell.into());
            }
        }
    };

    let shell = shell_type.build();

    println!("Finishing proto installation...");

    let content = format_exports(
        &shell,
        "proto",
        vec![
            Export::Var("PROTO_HOME".into(), "$HOME/.proto".into()),
            Export::Path(vec!["$PROTO_HOME/shims".into(), "$PROTO_HOME/bin".into()]),
        ],
    );

    let modified_profile_path = if args.no_modify_profile {
        None
    } else {
        update_shell_profile(&shell, &session, &content, interactive)?
    };

    #[allow(clippy::needless_bool)]
    let modified_system_env_path = if args.no_modify_path {
        false
    } else {
        #[cfg(windows)]
        {
            windows::do_add_to_path(vec![
                proto.env.store.shims_dir.clone(),
                proto.env.store.bin_dir.clone(),
            ])?
        }

        #[cfg(unix)]
        true
    };

    finished_message(
        &session,
        content,
        modified_profile_path,
        modified_system_env_path,
    );

    Ok(())
}

fn update_shell_profile(
    shell: &BoxedShell,
    session: &ProtoSession,
    content: &str,
    interactive: bool,
) -> miette::Result<Option<PathBuf>> {
    debug!("Updating PATH in {} shell", shell);

    let profile_path;

    if interactive {
        debug!("Prompting the user to select a shell profile");

        profile_path = prompt_for_shell_profile(shell, &session.env.cwd, &session.env.home)?;

        if let Some(profile) = &profile_path {
            debug!("Selected profile {}, updating", color::path(profile));

            write_profile(profile, content, "PROTO_HOME")?;
        }
    } else {
        debug!("Attempting to find a shell profile to update");

        profile_path = write_profile_if_not_setup(shell, content, "PROTO_HOME", &session.env.home)?;
    }

    // If we found a profile, update the global config so we can reference it
    if let Some(profile) = &profile_path {
        session.env.store.save_preferred_profile(profile)?;
    }

    Ok(profile_path)
}

fn help_message() {
    println!(
        "Need help? Join our Discord {}",
        color::url("https://discord.gg/qCh9MEynv2")
    );
}

fn already_setup_message(session: &ProtoSession) {
    let installed_bin_path = session.env.store.bin_dir.join(get_exe_file_name("proto"));

    println!(
        "Successfully installed proto to {}!",
        color::path(installed_bin_path),
    );
    help_message();
}

fn manual_system_path_message(session: &ProtoSession) {
    println!(
        "Add the following to your {} to get started:",
        color::property("PATH")
    );
    println!();
    println!(
        "{}",
        if cfg!(windows) {
            // We avoid %USERPROFILE% as it only works in the user path and not system path
            color::muted_light(format!(
                "{};{}",
                session.env.store.shims_dir.to_string_lossy(),
                session.env.store.bin_dir.to_string_lossy()
            ))
        } else {
            color::muted_light("$HOME/.proto/shims:$HOME/.proto/bin")
        },
    );
}

fn finished_message(
    session: &ProtoSession,
    exported_content: String,
    modified_profile_path: Option<PathBuf>,
    modified_system_env_path: bool,
) {
    let installed_bin_path = session.env.store.bin_dir.join(get_exe_file_name("proto"));

    println!(
        "Successfully installed proto to {}!",
        color::path(installed_bin_path),
    );

    modified_profile_path.as_ref().inspect(|path| {
        println!("The shell profile at {} was updated.", color::path(path));
    });

    if modified_system_env_path || modified_profile_path.is_some() {
        println!("Launch a new terminal window to start using proto!");
    } else if !exported_content.is_empty() && modified_profile_path.is_none() {
        if cfg!(windows) {
            manual_system_path_message(session);
            println!();
            println!("Or alternatively add the following to your shell profile:");
        } else {
            println!("Add the following to your shell profile to get started:");
        }
        println!();
        println!("{}", color::muted_light(exported_content.trim()));
    } else {
        manual_system_path_message(session);
    }

    println!();
    help_message();
}
