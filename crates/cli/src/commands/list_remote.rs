use crate::session::{LoadToolOptions, ProtoSession};
use clap::Args;
use iocraft::prelude::element;
use proto_core::Id;
use starbase::AppResult;
use starbase_console::ui::*;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct ListRemoteArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(long, help = "Include remote aliases in the output")]
    aliases: bool,
}

#[tracing::instrument(skip_all)]
pub async fn list_remote(session: ProtoSession, args: ListRemoteArgs) -> AppResult {
    let tool = session
        .load_tool_with_options(
            &args.id,
            LoadToolOptions {
                inherit_remote: true,
                ..Default::default()
            },
        )
        .await?;

    debug!("Loading versions from remote");

    if tool.remote_versions.is_empty() {
        session.console.render(element! {
            Notice(variant: Variant::Failure) {
                StyledText(
                    content: "No versions available from remote registry"
                )
            }
        })?;

        return Ok(Some(1));
    }

    session.console.out.write_line(
        tool.remote_versions
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    )?;

    if args.aliases && !tool.remote_aliases.is_empty() {
        session.console.out.write_line(
            tool.remote_aliases
                .iter()
                .map(|(k, v)| format!("{k} -> {v}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )?;
    }

    Ok(None)
}
