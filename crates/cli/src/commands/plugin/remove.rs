use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{Id, PROTO_CONFIG_NAME, PinLocation, ProtoConfig};
use starbase::AppResult;
use starbase_console::ui::*;

#[derive(Args, Clone, Debug)]
pub struct RemovePluginArgs {
    #[arg(required = true, help = "ID of plugin")]
    id: Id,

    #[arg(long, default_value_t, help = "Location of .prototools to remove from")]
    from: PinLocation,
}

#[tracing::instrument(skip_all)]
pub async fn remove(session: ProtoSession, args: RemovePluginArgs) -> AppResult {
    let config_dir = session.env.get_config_dir(args.from);
    let config_path = config_dir.join(PROTO_CONFIG_NAME);

    if !config_path.exists() {
        return Err(ProtoCliError::MissingToolsConfigInCwd { path: config_path }.into());
    }

    let config_path = ProtoConfig::update_document(config_dir, |doc| {
        if let Some(plugins) = doc["plugins"].as_table_mut() {
            plugins.remove(&args.id);

            if plugins.is_empty() {
                doc.as_table_mut().remove("plugins");
            }
        }

        if let Some(tools) = doc["tools"].as_table_mut() {
            tools.remove(&args.id);

            if tools.is_empty() {
                doc.as_table_mut().remove("tools");
            }
        }

        doc.as_table_mut().remove(&args.id);

        // if let Some(plugins) = &mut config.plugins {
        //     plugins.remove(&args.id);
        // }

        // if let Some(tools) = &mut config.tools {
        //     tools.remove(&args.id);
        // }

        // if let Some(versions) = &mut config.versions {
        //     versions.remove(&args.id);
        // }
    })?;

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: format!(
                    "Removed <id>{}</id> plugin from config <path>{}</path>",
                    args.id,
                    config_path.display(),
                ),
            )
        }
    })?;

    Ok(None)
}
