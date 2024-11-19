use crate::helpers::{map_pin_type, PinOption};
use crate::session::ProtoSession;
use clap::Args;
use proto_core::{load_tool_from_locator, Id, PluginLocator, ProtoConfig};
use starbase::AppResult;
use starbase_styles::color::{self, apply_style_tags};

#[derive(Args, Clone, Debug)]
pub struct AddPluginArgs {
    #[arg(required = true, help = "ID of plugin")]
    id: Id,

    #[arg(required = true, help = "Locator string to find and load the plugin")]
    plugin: PluginLocator,

    #[arg(long, group = "pin", help = "Add to the global ~/.proto/.prototools")]
    global: bool,

    #[arg(long, group = "pin", help = "Location of .prototools to add to")]
    to: Option<PinOption>,
}

#[tracing::instrument(skip_all)]
pub async fn add(session: ProtoSession, args: AddPluginArgs) -> AppResult {
    let config_path = ProtoConfig::update(
        session
            .env
            .get_config_dir(map_pin_type(args.global, args.to)),
        |config| {
            config
                .plugins
                .get_or_insert(Default::default())
                .insert(args.id.clone(), args.plugin.clone());
        },
    )?;

    // Load the tool and verify it works. We can't load the tool with the
    // session as the config has already been cached, and doesn't reflect
    // the recent addition!
    let tool = load_tool_from_locator(&args.id, &session.env, &args.plugin).await?;

    if !tool.metadata.deprecations.is_empty() {
        let mut output = color::caution("Deprecation notices from the plugin:\n");

        for msg in &tool.metadata.deprecations {
            output.push_str("  ");
            output.push_str(&color::muted("-"));
            output.push_str(" ");
            output.push_str(&apply_style_tags(msg));
            output.push('\n');
        }

        println!("{output}");
    }

    println!(
        "Added plugin {} to config {}",
        color::id(&args.id),
        color::path(config_path)
    );

    Ok(None)
}
