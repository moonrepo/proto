use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{Id, PinLocation, PluginLocator, PluginType, ProtoConfig, cfg};
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

    #[arg(long = "type", default_value_t, help = "The type of plugin to add")]
    ty: PluginType,
}

#[tracing::instrument(skip_all)]
pub async fn add(session: ProtoSession, args: AddPluginArgs) -> AppResult {
    let config_path = ProtoConfig::update_document(session.env.get_config_dir(args.to), |doc| {
        let key = if args.ty == PluginType::Backend {
            "backends"
        } else {
            "tools"
        };

        // Convert legacy [plugins] to [plugins.tools]
        if doc.contains_key("plugins")
            && doc["plugins"].as_table().is_some_and(|table| {
                !table.contains_key("backends") && !table.contains_key("tools")
            })
        {
            let existing = doc["plugins"].clone();

            doc.remove("plugins");

            let plugins = doc["plugins"].or_insert(cfg::implicit_table());
            plugins["tools"] = existing;
        }

        // Add plugin to nested tables
        let plugins = doc["plugins"].or_insert(cfg::implicit_table());
        let table = plugins[key].or_insert(cfg::table());
        table[args.id.as_str()] = cfg::value(args.plugin.to_string());
    })?;

    // Load the tool and verify it works. We can't load the tool with the
    // session as the config has already been cached, and doesn't reflect
    // the recent addition!
    #[cfg(not(debug_assertions))]
    {
        use proto_core::ToolContext;

        let tool = proto_core::load_tool_from_locator(
            ToolContext::parse(&args.id)?,
            &session.env,
            &args.plugin,
        )
        .await?;

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
