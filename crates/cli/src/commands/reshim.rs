use crate::helpers::ProtoResource;
use clap::Args;
use proto_core::remove_bin_file;
use starbase::system;
use starbase_utils::fs;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct ReshimArgs {
    #[arg(long, help = "Also recreate binary symlinks")]
    bin: bool,
}

#[system]
pub async fn reshim(args: ArgsRef<ReshimArgs>, proto: ResourceRef<ProtoResource>) {
    if args.bin {
        info!("Regenerating bins and shims...");
    } else {
        info!("Regenerating shims...");
    }

    // Delete all shims
    fs::remove_dir_all(&proto.env.shims_dir)?;

    // Delete all bins (except for proto)
    if args.bin {
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
    for mut tool in proto.load_tools().await? {
        debug!("Regenerating {}", tool.get_name());

        tool.create_executables(true, args.bin).await?;
    }

    info!("Regeneration complete!");
}
