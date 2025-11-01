use crate::mcp::ProtoMcp;
use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use rmcp::{ServerHandler, ServiceExt, transport::stdio};
use starbase::AppResult;
use starbase_console::ui::*;

#[derive(Args, Clone, Debug)]
pub struct McpArgs {
    #[arg(
        long,
        help = "Display server information and list available tools and resources"
    )]
    info: bool,
}

#[tracing::instrument(skip_all)]
pub async fn mcp(session: ProtoSession, args: McpArgs) -> AppResult {
    let console = session.console.clone();
    let server = ProtoMcp::new(session);

    if !args.info {
        server
            .serve(stdio())
            .await
            .into_diagnostic()?
            .waiting()
            .await
            .into_diagnostic()?;

        return Ok(None);
    }

    let info = server.get_info();

    let mut tools = server.tool_router.list_all();
    tools.sort_by(|a, d| a.name.cmp(&d.name));

    let mut resources = server.list_all_resources().resources;
    resources.sort_by(|a, d| a.name.cmp(&d.name));

    console.render(element! {
        Container {
            Section(title: "Server") {
                #(info.instructions.as_ref().map(|desc| {
                    element! {
                        View(margin_bottom: 1) {
                            StyledText(
                                content: desc,
                            )
                        }
                    }
                }))

                Entry(
                    name: "CLI version",
                    value: element! {
                        StyledText(
                            content: info.server_info.version.to_string(),
                            style: Style::Symbol
                        )
                    }.into_any()
                )
                Entry(
                    name: "Protocol version",
                    value: element! {
                        StyledText(
                            content: info.protocol_version.to_string(),
                            style: Style::Hash
                        )
                    }.into_any()
                )
            }

            Section(title: "Tools") {
                #(tools.into_iter().map(|tool| {
                    element! {
                        Entry(
                            name: tool.name.to_string(),
                            content: tool.description.unwrap_or_default().to_string(),
                        )
                    }
                }))
            }

            Section(title: "Resources") {
                #(resources.into_iter().map(|resource| {
                    element! {
                        Entry(
                            name: resource.uri.to_string(),
                            content: resource.name.to_string(),
                        )
                    }
                }))
            }
        }
    })?;

    Ok(None)
}
