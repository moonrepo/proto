use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::fs;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct RegenArgs {
    #[arg(long, help = "Also recreate binary symlinks")]
    bin: bool,
}

#[tracing::instrument(skip_all)]
pub async fn regen(session: ProtoSession, args: RegenArgs) -> AppResult {
    let store = &session.env.store;
    let progress = session.render_progress_loader().await;

    progress.set_message(if args.bin {
        "Regenerating shims and bins..."
    } else {
        "Regenerating shims..."
    });

    // Delete all shims
    progress.set_message("Removing old shims");

    fs::remove_dir_all(&store.shims_dir)?;

    // Delete all bins (except for proto)
    if args.bin {
        progress.set_message("Removing old bins");

        for file in fs::read_dir_all(&store.bin_dir)? {
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

            store.unlink_bin(&path)?;
        }
    }

    // Regenerate everything!
    let config = session.env.load_config()?;

    for mut tool in session.load_tools().await? {
        progress.set_message(format!("Regenerating {}", tool.get_name()));

        // Shims - Create once if tool has a configured version
        if let Some(version) = config.versions.get(&tool.context) {
            debug!("Regenerating {} shim", tool.get_name());

            tool.resolve_version(version, true).await?;
            tool.generate_shims(true).await?;
        }

        // Bins - Create for each installed version
        if args.bin {
            debug!("Relinking {} bin", tool.get_name());

            tool.symlink_bins(true).await?;
        }
    }

    progress.stop().await?;

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: if args.bin {
                    "Regenerated shims and bins!"
                } else {
                    "Regenerated shims!"
                },
            )
        }
    })?;

    Ok(None)
}
