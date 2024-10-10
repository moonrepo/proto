use crate::helpers::{map_pin_type, PinOption};
use crate::session::ProtoSession;
use clap::Args;
use proto_core::{Id, ProtoConfig};
use starbase::AppResult;
use starbase_styles::color;

#[derive(Args, Clone, Debug)]
pub struct UnpinArgs {
    #[arg(required = true, help = "ID of tool")]
    pub id: Id,

    #[arg(
        long,
        group = "pin",
        help = "Unpin from the global ~/.proto/.prototools"
    )]
    pub global: bool,

    #[arg(long, group = "pin", help = "Location of .prototools to unpin from")]
    pub from: Option<PinOption>,
}

#[tracing::instrument(skip_all)]
pub async fn unpin(session: ProtoSession, args: UnpinArgs) -> AppResult {
    let tool = session.load_tool(&args.id).await?;
    let mut value = None;

    let config_path = ProtoConfig::update(
        tool.proto
            .get_config_dir(map_pin_type(args.global, args.from)),
        |config| {
            if let Some(versions) = &mut config.versions {
                value = versions.remove(&tool.id);
            }

            // Remove these also just in case
            if let Some(versions) = &mut config.unknown {
                versions.remove(tool.id.as_str());
            }
        },
    )?;

    let Some(value) = value else {
        eprintln!("No version pinned in config {}", color::path(config_path));

        return Ok(Some(1));
    };

    println!(
        "Removed version {} from config {}",
        color::hash(value.to_string()),
        color::path(config_path)
    );

    Ok(None)
}
