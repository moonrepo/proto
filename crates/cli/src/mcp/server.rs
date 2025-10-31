use super::resources::*;
use super::tools::*;
use crate::session::ProtoSession;
use proto_core::get_proto_version;
use proto_core::{ProtoConfigEnvOptions, ToolContext, ToolSpec};
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
use serde_json::json;
use std::fmt::Display;

#[derive(Clone)]
pub struct ProtoMcp {
    session: ProtoSession,
    tool_router: ToolRouter<ProtoMcp>,
    // prompt_router: PromptRouter<ProtoMcp>,
}

impl ProtoMcp {
    fn resource_config(&self) -> miette::Result<ConfigResource<'_>> {
        let env = &self.session.env;

        Ok(ConfigResource {
            working_dir: env.working_dir.clone(),
            config_mode: env.config_mode,
            config_files: env
                .load_file_manager()?
                .entries
                .iter()
                .map(|entry| &entry.path)
                .collect(),
            config: env.load_config()?,
        })
    }

    fn resource_env(&self) -> miette::Result<EnvResource<'_>> {
        let env = &self.session.env;
        let config = env.load_config()?;
        let options = ProtoConfigEnvOptions {
            include_shared: true,
            ..Default::default()
        };

        Ok(EnvResource {
            working_dir: env.working_dir.clone(),
            store_dir: env.store.dir.clone(),
            env_mode: env.env_mode.clone(),
            env_files: config.get_env_files(options.clone()),
            env_vars: config.get_env_vars(options)?,
            proto_version: get_proto_version().to_string(),
            system_arch: env.arch,
            system_os: env.os,
        })
    }

    async fn resource_tools(&self) -> miette::Result<ToolsResource> {
        let mut resource = ToolsResource {
            tools: Default::default(),
        };

        for tool in self.session.load_tools().await? {
            resource.tools.insert(
                tool.context.clone(),
                ToolResourceEntry {
                    tool_dir: tool.get_inventory_dir().to_path_buf(),
                    installed_versions: Vec::from_iter(
                        tool.inventory.manifest.installed_versions.clone(),
                    ),
                },
            );
        }

        Ok(resource)
    }
}

#[tool_router]
impl ProtoMcp {
    pub fn new(session: ProtoSession) -> Self {
        Self {
            session,
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
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation {
                website_url: Some("https://moonrepo.dev/proto".into()),
                ..Implementation::from_build_env()
            },
            instructions: Some("The proto MCP server provides resources and tools for managing your toolchain, environment, and more.".to_string()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                RawResource::new("proto://config", "Configuration".to_string()).no_annotation(),
                RawResource::new("proto://env", "Environment".to_string()).no_annotation(),
                RawResource::new("proto://tools", "Installed tools".to_string()).no_annotation(),
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let text = match uri.as_str() {
            "proto://config" => {
                let resource = self
                    .resource_config()
                    .map_err(|error| map_resource_error(error, &uri))?;

                serde_json::to_string_pretty(&resource).unwrap()
            }
            "proto://env" => {
                let resource = self
                    .resource_env()
                    .map_err(|error| map_resource_error(error, &uri))?;

                serde_json::to_string_pretty(&resource).unwrap()
            }
            "proto://tools" => {
                let resource = self
                    .resource_tools()
                    .await
                    .map_err(|error| map_resource_error(error, &uri))?;

                serde_json::to_string_pretty(&resource).unwrap()
            }
            _ => {
                return Err(McpError::resource_not_found(
                    "Resource does not exist.",
                    Some(json!({
                        "uri": uri
                    })),
                ));
            }
        };

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::TextResourceContents {
                uri,
                text,
                mime_type: Some("application/json".into()),
                meta: None,
            }],
        })
    }
}

fn map_parse_error(error: impl Display) -> McpError {
    McpError::parse_error(error.to_string(), None)
}

fn map_resource_error(error: impl Display, uri: &str) -> McpError {
    McpError::internal_error(
        error.to_string(),
        Some(json!({
            "uri": uri
        })),
    )
}

// uninstall_tool
// list_tool_versions
