use super::tools::*;
use proto_core::{ProtoEnvironment, ToolContext, ToolSpec};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::{Json, Parameters},
    },
    model::*,
    prompt, prompt_handler, prompt_router, schemars,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use std::{fmt::Display, sync::Arc};

#[derive(Clone)]
pub struct ProtoMcp {
    env: Arc<ProtoEnvironment>,
    tool_router: ToolRouter<ProtoMcp>,
    // prompt_router: PromptRouter<ProtoMcp>,
}

#[tool_router]
impl ProtoMcp {
    pub fn new(env: Arc<ProtoEnvironment>) -> Self {
        Self {
            env,
            tool_router: Self::tool_router(),
            // prompt_router: Self::prompt_router(),
        }
    }

    #[tool(description = "Install a tool with a specification.")]
    async fn install_tool(
        &self,
        params: Parameters<InstallToolRequest>,
    ) -> Result<Json<InstallToolResponse>, McpError> {
        let req = params.0;
        let context = ToolContext::parse(&req.tool).map_err(map_parse_error)?;
        let spec =
            ToolSpec::parse(req.spec.as_deref().unwrap_or("latest")).map_err(map_parse_error)?;

        dbg!(&context, &spec);

        Ok(Json(InstallToolResponse {
            installed: false,
            spec: spec.to_string(),
        }))
    }
}

#[tool_handler]
impl ServerHandler for ProtoMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder()
                // .enable_prompts()
                // .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation {
                website_url: Some("https://moonrepo.dev/proto".into()),
                ..Implementation::from_build_env()
            },
            instructions: Some("The proto MCP server provides resources and tools for managing your toolchain, environment, and more.".to_string()),
        }
    }
}

fn map_parse_error(error: impl Display) -> McpError {
    McpError::parse_error(error.to_string(), None)
}

// uninstall_tool
// list_tool_versions
