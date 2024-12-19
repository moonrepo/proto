use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{Id, PinLocation, PluginLocator, ProtoConfig};
use starbase::AppResult;
use starbase_console::ui::*;

#[derive(Args, Clone, Debug)]
pub struct AddPluginArgs {
    #[arg(required = true, help = "ID of plugin")]
    id: Id,

    #[arg(required = true, help = "Locator string to find and load the plugin")]
    plugin: PluginLocator,

    #[arg(long, default_value_t, help = "Location of .prototools to add to")]
    to: PinLocation,
}

#[tracing::instrument(skip_all)]
pub async fn add(session: ProtoSession, args: AddPluginArgs) -> AppResult {
    let config_path = ProtoConfig::update(session.env.get_config_dir(args.to), |config| {
        config
            .plugins
            .get_or_insert(Default::default())
            .insert(args.id.clone(), args.plugin.clone());
    })?;

    // Load the tool and verify it works. We can't load the tool with the
    // session as the config has already been cached, and doesn't reflect
    // the recent addition!
    #[cfg(not(debug_assertions))]
    {
        let tool = proto_core::load_tool_from_locator(&args.id, &session.env, &args.plugin).await?;

        if !tool.metadata.deprecations.is_empty() {
            session.console.render(element! {
                Notice(title: "Deprecations".to_owned(), variant: Variant::Info) {
                    List {
                        #(tool.metadata.deprecations.iter().map(|message| {
                            element! {
                                ListItem {
                                    StyledText(content: message)
                                }
                            }
                        }))
                    }
                }
            })?;
        }
    }

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: format!(
                    "Added <id>{}</id> plugin to config <path>{}</path>",
                    args.id,
                    config_path.display(),
                ),
            )
        }
    })?;

    Ok(None)
}
