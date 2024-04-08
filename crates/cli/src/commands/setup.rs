use crate::helpers::ProtoResource;
use crate::shell::{
    detect_shell, format_exports, prompt_for_shell_profile, write_profile,
    write_profile_if_not_setup, Export,
};
use clap::Args;
use clap_complete::Shell;
use proto_shim::get_exe_file_name;
use starbase::system;
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
    shell: Option<Shell>,

    #[arg(long, help = "Don't update a shell profile")]
    no_profile: bool,

    #[arg(long, help = "Don't update the system path")]
    no_modify_path: bool,

    // deprecated
    #[arg(long, hide = true, help = "Return the shell profile path if setup")]
    profile: bool,

    #[arg(long, short = 'y', help = "Avoid interactive prompts and use defaults")]
    yes: bool,
}

#[system]
pub async fn setup(args: ArgsRef<SetupArgs>, proto: ResourceRef<ProtoResource>) {
    let shell = detect_shell(args.shell);
    let paths = env::split_paths(&env::var("PATH").unwrap()).collect::<Vec<_>>();

    if paths.contains(&proto.env.store.shims_dir) && paths.contains(&proto.env.store.bin_dir) {
        debug!("Skipping setup, proto already exists in PATH");

        already_setup_message(&proto);

        return Ok(());
    }

    println!("Finishing proto installation...");

    let content = format_exports(
        &shell,
        "proto",
        vec![
            Export::Var("PROTO_HOME".into(), "$HOME/.proto".into()),
            Export::Path(vec!["$PROTO_HOME/shims".into(), "$PROTO_HOME/bin".into()]),
        ],
    );

    let profile_path = match content {
        Some(ref content) if !args.no_profile => {
            let interactive = !args.yes && env::var("CI").is_err() && stdout().is_terminal();

            update_shell_profile(&shell, &proto, &content, interactive)?
        }
        _ => None,
    };

    #[cfg(windows)]
    let system_path_was_updated = if !args.no_modify_path {
        windows::do_add_to_path(vec![
            proto.env.store.shims_dir.clone(),
            proto.env.store.bin_dir.clone(),
        ])?
    } else {
        false
    };

    // unix makes no distinction here
    #[cfg(unix)]
    let system_path_was_updated = profile_path.is_some();

    finished_message(&proto, content, profile_path, system_path_was_updated);
}

fn update_shell_profile(
    shell: &Shell,
    proto: &ProtoResource,
    content: &String,
    interactive: bool,
) -> miette::Result<Option<PathBuf>> {
    debug!("Updating PATH in {} shell", shell);

    let profile_path;

    // If interactive, let the user pick a profile
    if interactive {
        debug!("Prompting the user to select a shell profile");

        profile_path = prompt_for_shell_profile(shell, &proto.env.cwd)?;

        if let Some(profile) = &profile_path {
            debug!("Selected profile {}, updating", color::path(profile));

            write_profile(profile, &content, "PROTO_HOME")?;
        }
    }
    // Otherwise attempt to find one
    else {
        debug!("Attempting to find a shell profile to update");

        profile_path = write_profile_if_not_setup(&shell, &content, "PROTO_HOME")?;
    }

    // If we found a profile, update the global config so we can reference it
    if let Some(profile) = &profile_path {
        proto.env.store.save_preferred_profile(profile)?;
    }

    Ok(profile_path)
}

fn help_message() {
    println!(
        "Need help? Join our Discord {}",
        color::url("https://discord.gg/qCh9MEynv2")
    );
}

fn already_setup_message(proto: &ProtoResource) {
    let installed_bin_path = proto.env.store.bin_dir.join(get_exe_file_name("proto"));

    println!(
        "Successfully installed proto to {}!",
        color::path(installed_bin_path),
    );
    help_message();
}

fn manual_system_path_message(proto: &ProtoResource) {
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
                proto.env.store.shims_dir.to_string_lossy(),
                proto.env.store.bin_dir.to_string_lossy()
            ))
        } else {
            color::muted_light("$HOME/.proto/shims:$HOME/.proto/bin")
        },
    );
}

fn finished_message(
    proto: &ProtoResource,
    exported_content: Option<String>,
    profile_path: Option<PathBuf>,
    system_path_was_updated: bool,
) {
    let installed_bin_path = proto.env.store.bin_dir.join(get_exe_file_name("proto"));

    println!(
        "Successfully installed proto to {}!",
        color::path(installed_bin_path),
    );

    profile_path.as_ref().inspect(|path| {
        println!("The shell profile at {} was updated.", color::path(path));
    });

    if system_path_was_updated || profile_path.is_some() {
        println!("Launch a new terminal window to start using proto!");
    } else if exported_content.is_some() && profile_path.is_none() {
        if cfg!(windows) {
            manual_system_path_message(proto);
            println!();
            println!("Or alternatively add the following to your shell profile:");
        } else {
            println!("Add the following to your shell profile to get started:");
        }
        println!();
        println!("{}", color::muted_light(exported_content.unwrap().trim()));
    } else {
        manual_system_path_message(&proto);
    }

    println!();
    help_message();
}
