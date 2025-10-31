use crate::mcp::ProtoMcp;
use crate::session::ProtoSession;
use clap::Args;
use miette::IntoDiagnostic;
use rmcp::{ServiceExt, transport::stdio};
use starbase::AppResult;

#[derive(Args, Clone, Debug)]
pub struct McpArgs {}

#[tracing::instrument(skip_all)]
pub async fn mcp(session: ProtoSession, _args: McpArgs) -> AppResult {
    let service = ProtoMcp::new(session.env.clone())
        .serve(stdio())
        .await
        .into_diagnostic()?;

    service.waiting().await.into_diagnostic()?;

    Ok(None)
}
