use crate::mcp::ProtoMcp;
use crate::session::{ProtoSession, SessionResult};
use clap::Args;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use rmcp::model::{InitializeResult, ProtocolVersion};
use rmcp::{ServerHandler, ServiceExt, transport::stdio};
use serde::Serialize;
use starbase_console::ui::*;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct McpArgs {
    #[arg(
        long,
        help = "Display server information and list available tools and resources"
    )]
    pub info: bool,
}

#[derive(Serialize)]
pub struct McpOutput {
    info: InitializeResult,
    protocol_versions: Vec<ProtocolVersion>,
    tools: Vec<rmcp::model::Tool>,
    resources: Vec<rmcp::model::Resource>,
}

#[instrument(skip(session))]
pub async fn mcp(session: ProtoSession, args: McpArgs) -> SessionResult {
    let console = session.console.clone();
    let server = ProtoMcp::new(session.clone());

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

    // The version in `info` is only what we fall back to when a client requests
    // a version we don't support, so report everything we can negotiate to
    let mut protocol_versions = server.supported_protocol_versions().into_owned();
    protocol_versions.sort_by(|a, d| a.as_str().cmp(d.as_str()));

    let mut tools = server.tool_router.list_all();
    tools.sort_by(|a, d| a.name.cmp(&d.name));

    let mut resources = server.list_all_resources().resources;
    resources.sort_by(|a, d| a.name.cmp(&d.name));

    if session.is_json_format() {
        console.write_json_for_format(McpOutput {
            info,
            protocol_versions,
            tools,
            resources,
        })?;

        return Ok(None);
    }

    let protocol_versions = protocol_versions
        .iter()
        .map(|version| {
            if *version == info.protocol_version {
                format!("<hash>{version}</hash> <mutedlight>(default)</mutedlight>")
            } else {
                format!("<hash>{version}</hash>")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

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
                    name: "Protocol versions",
                    value: element! {
                        StyledText(
                            content: protocol_versions,
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
