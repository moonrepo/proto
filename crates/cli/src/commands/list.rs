use crate::session::{LoadToolOptions, ProtoSession};
use clap::Args;
use iocraft::prelude::element;
use proto_core::Id;
use starbase::AppResult;
use starbase_console::ui::*;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct ListArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(long, help = "Include local aliases in the output")]
    aliases: bool,
}

#[tracing::instrument(skip_all)]
pub async fn list(session: ProtoSession, args: ListArgs) -> AppResult {
    let tool = session
        .load_tool_with_options(
            &args.id,
            LoadToolOptions {
                inherit_local: true,
                ..Default::default()
            },
        )
        .await?;

    debug!(manifest = ?tool.inventory.manifest.path, "Using versions from manifest");

    if tool.installed_versions.is_empty() {
        session.console.render(element! {
            Notice(variant: Variant::Failure) {
                StyledText(
                    content: format!(
                        "No versions installed locally, try installing the latest version with <shell>proto install {}</shell>",
                        args.id
                    ),
                )
            }
        })?;

        return Ok(Some(1));
    }

    session.console.out.write_line(
        tool.installed_versions
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    )?;

    if args.aliases && !tool.local_aliases.is_empty() {
        session.console.out.write_line(
            tool.local_aliases
                .iter()
                .map(|(k, v)| format!("{k} -> {v}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )?;
    }

    Ok(None)
}
