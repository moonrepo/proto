use crate::helpers::ProtoResource;
use crate::shell::{
    detect_shell, find_profiles, format_exports, write_profile, write_profile_if_not_setup, Export,
};
use clap::Args;
use clap_complete::Shell;
use dialoguer::{Input, Select};
use miette::IntoDiagnostic;
use proto_shim::get_exe_file_name;
use starbase::system;
use starbase_styles::color;
use std::env;
use std::path::PathBuf;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct SetupArgs {
    #[arg(long, help = "Shell to setup for")]
    shell: Option<Shell>,

    #[arg(long, help = "Don't update a shell profile")]
    no_profile: bool,

    // deprecated
    #[arg(long, help = "Return the shell profile path if setup")]
    profile: bool,

    #[arg(long, help = "Avoid interactive prompts and use defaults")]
    yes: bool,
}

#[system]
pub async fn setup(args: ArgsRef<SetupArgs>, proto: ResourceRef<ProtoResource>) {
    let shell = detect_shell(args.shell);

    let paths = env::var("PATH").expect("Missing PATH!");
    let paths = env::split_paths(&paths).collect::<Vec<_>>();

    if paths.contains(&proto.env.shims_dir) && paths.contains(&proto.env.bin_dir) {
        debug!("Skipping setup, PROTO_HOME already exists in PATH");

        return Ok(());
    }

    let installed_bin_path = env::var("PROTO_INSTALL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| proto.env.home.join("bin"))
        .join(get_exe_file_name("proto"));

    let exports = vec![
        Export::Var("PROTO_HOME".into(), "$HOME/.proto".into()),
        Export::Path(vec!["$PROTO_HOME/shims".into(), "$PROTO_HOME/bin".into()]),
    ];

    let Some(content) = format_exports(&shell, "proto", exports) else {
        finished_message(installed_bin_path, None, None);

        return Ok(());
    };

    // Avoid updating the shell profile
    if args.no_profile {
        finished_message(installed_bin_path, None, Some(content));

        return Ok(());
    }

    // Otherwise attempt to update the shell profile
    debug!("Updating PATH in {} shell", shell);

    let mut profile_path = None;
    let interactive = !args.yes && env::var("CI").is_err();

    // If interactive, let the user pick a profile
    if interactive {
        debug!("Prompting the user to select a shell profile");

        let profiles = find_profiles(&shell)?;
        let default_index = profiles.len() - 1;
        let other_index = default_index + 1;
        let none_index = other_index + 1;

        let mut items = profiles.iter().map(color::path).collect::<Vec<_>>();
        items.push("Other".to_owned());
        items.push("None".to_owned());

        let selected_index = Select::new()
            .with_prompt("Which profile to update?")
            .items(&items)
            .default(default_index)
            .interact_opt()
            .into_diagnostic()?
            .unwrap_or(default_index);

        if selected_index == none_index {
            profile_path = None;
        } else if selected_index == other_index {
            let custom_path = PathBuf::from(
                Input::<String>::new()
                    .with_prompt("Custom profile path?")
                    .interact_text()
                    .into_diagnostic()?,
            );

            profile_path = Some(if custom_path.is_absolute() {
                custom_path
            } else {
                proto.env.cwd.join(custom_path)
            });
        } else {
            profile_path = Some(profiles[selected_index].clone());
        }

        if let Some(profile) = &profile_path {
            debug!("Selected profile {}, updating", color::path(profile));

            write_profile(profile, &content, "PROTO_HOME")?;
        }
    }
    // Otherwise attempt to find one
    else {
        debug!("Attempting to find a shell profile to update");

        if let Some(updated_profile) = write_profile_if_not_setup(&shell, &content, "PROTO_HOME")? {
            profile_path = Some(updated_profile);
        }
    }

    finished_message(installed_bin_path, profile_path, Some(content));
}

fn finished_message(
    installed_bin_path: PathBuf,
    updated_profile_path: Option<PathBuf>,
    exported_content: Option<String>,
) {
    if let Some(profile_path) = updated_profile_path {
        println!(
            "Successfully installed proto to {} and updated profile {}",
            color::path(installed_bin_path),
            color::path(profile_path),
        );
        println!("Launch a new terminal window to start using proto!");
    } else {
        println!(
            "Successfully installed proto to {}",
            color::path(installed_bin_path),
        );

        if let Some(content) = exported_content {
            println!("Add the following to your shell profile to get started:");
            println!();
            println!("{}", color::muted_light(content));
        } else {
            println!(
                "Add the following to your {} to get started:",
                color::property("PATH")
            );
            println!();
            println!(
                "{}",
                color::muted_light("$HOME/.proto/shims;$HOME/.proto/bin")
            );
        }
    }

    println!();
    println!(
        "Need help? Join our Discord {}",
        color::url("https://discord.gg/qCh9MEynv2")
    );
}
