use crate::helpers::ProtoResource;
use clap::Args;
use proto_core::remove_bin_file;
use starbase::system;
use starbase_utils::fs;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct RegenArgs {
    #[arg(long, help = "Also recreate binary symlinks")]
    bin: bool,
}

#[system]
pub async fn regen(args: ArgsRef<RegenArgs>, proto: ResourceRef<ProtoResource>) {
    if args.bin {
        info!("Regenerating bins and shims...");
    } else {
        info!("Regenerating shims...");
    }

    // Delete all shims
    debug!("Removing old shims");

    fs::remove_dir_all(&proto.env.shims_dir)?;

    // Delete all bins (except for proto)
    if args.bin {
        debug!("Removing old bins");

        for file in fs::read_dir_all(&proto.env.bin_dir)? {
            let path = file.path();
            let name = fs::file_name(&path);

            if path.is_dir()
                || name == "proto"
                || name == "proto.exe"
                || name == "proto-shim"
                || name == "proto-shim.exe"
            {
                continue;
            }

            remove_bin_file(path)?;
        }
    }

    // Regenerate everything!
    debug!("Loading tools");

    let tools = proto.load_tools().await?;
    let config = proto.env.load_config()?;

    for mut tool in tools {
        if let Some(version) = config.versions.get(&tool.id) {
            tool.resolve_version(version, true).await?;
        } else {
            continue;
        }

        debug!("Regenerating {}", tool.get_name());

        tool.create_executables(true, args.bin).await?;
    }

    info!("Regeneration complete!");
}
