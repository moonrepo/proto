use crate::session::ProtoSession;
use clap::Args;
use starbase::AppResult;
use starbase_utils::fs;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct RegenArgs {
    #[arg(long, help = "Also recreate binary symlinks")]
    bin: bool,
}

#[tracing::instrument(skip_all)]
pub async fn regen(session: ProtoSession, args: RegenArgs) -> AppResult {
    if args.bin {
        println!("Regenerating bins and shims...");
    } else {
        println!("Regenerating shims...");
    }

    // Delete all shims
    debug!("Removing old shims");

    fs::remove_dir_all(&session.env.store.shims_dir)?;

    // Delete all bins (except for proto)
    if args.bin {
        debug!("Removing old bins");

        for file in fs::read_dir_all(&session.env.store.bin_dir)? {
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

            session.env.store.unlink_bin(&path)?;
        }
    }

    // Regenerate everything!
    debug!("Loading tools");

    let config = session.env.load_config()?;

    for mut tool in session.load_tools().await? {
        // Shims - Create once if has a configured version.
        if config.versions.contains_key(&tool.id) {
            debug!("Regenerating {} shim", tool.get_name());

            tool.generate_shims(false).await?;
        }

        // Bins - Create for each installed version.
        if args.bin {
            debug!("Relinking {} bin", tool.get_name());

            for version in tool.inventory.manifest.installed_versions.clone() {
                let version = version.to_unresolved_spec();

                tool.version = None;
                tool.resolve_version(&version, true).await?;
                tool.symlink_bins(false).await?;
            }
        }
    }

    println!("Regeneration complete!");

    Ok(None)
}
