use crate::helpers::ProtoResource;
use crate::shell::{detect_shell, format_exports, write_profile_if_not_setup, Export};
use clap::Args;
use clap_complete::Shell;
use starbase::system;
use std::env;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct SetupArgs {
    #[arg(long, help = "Shell to setup for")]
    shell: Option<Shell>,

    #[arg(long, help = "Return the profile path if setup")]
    profile: bool,
}

#[system]
pub async fn setup(args: ArgsRef<SetupArgs>, proto: ResourceRef<ProtoResource>) {
    let shell = detect_shell(args.shell);

    let paths = env::var("PATH").expect("Missing PATH!");
    let paths = env::split_paths(&paths).collect::<Vec<_>>();

    if paths.contains(&proto.env.shims_dir) || paths.contains(&proto.env.bin_dir) {
        debug!("Skipping setup, PROTO_HOME already exists in PATH");

        return Ok(());
    }

    debug!("Updating PATH in {} shell", shell);

    let exports = vec![
        Export::Var("PROTO_HOME".into(), "$HOME/.proto".into()),
        Export::Path(vec!["$PROTO_HOME/shims".into(), "$PROTO_HOME/bin".into()]),
    ];

    if let Some(content) = format_exports(&shell, "proto", exports) {
        if let Some(updated_profile) = write_profile_if_not_setup(&shell, content, "PROTO_HOME")? {
            if args.profile {
                println!("{}", updated_profile.to_string_lossy());
            }
        }
    }
}
