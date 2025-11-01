use super::resources::*;
use super::tools::*;
use crate::session::ProtoSession;
use crate::workflows::*;
use proto_core::{
    PinLocation, ProtoConfigEnvOptions, ToolContext, ToolSpec, UnresolvedVersionSpec,
    get_proto_version,
};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde_json::json;
use std::fmt::Display;

macro_rules! handle_tool_error {
    ($result:expr) => {
        match $result {
            Ok(inner) => inner,
            Err(error) => {
                return Ok(CallToolResult::error(vec![Annotated::new(
                    RawContent::text(error.to_string()),
                    None,
                )]));
            }
        }
    };
}

#[derive(Clone)]
pub struct ProtoMcp {
    session: ProtoSession,

    pub tool_router: ToolRouter<ProtoMcp>,
}

impl ProtoMcp {
    pub fn list_all_resources(&self) -> ListResourcesResult {
        ListResourcesResult {
            resources: vec![
                RawResource::new("proto://config", "Configuration".to_string()).no_annotation(),
                RawResource::new("proto://env", "Environment".to_string()).no_annotation(),
                RawResource::new("proto://tools", "Installed tools".to_string()).no_annotation(),
            ],
            next_cursor: None,
        }
    }

    fn parse_context(&self, value: &str) -> Result<ToolContext, McpError> {
        if value.is_empty() {
            return Err(McpError::invalid_params(
                "Tool identifier/context required.",
                Some(json!({
                    "param": "tool"
                })),
            ));
        }

        ToolContext::parse(value).map_err(map_parse_error)
    }

    fn parse_spec(&self, value: &str) -> Result<ToolSpec, McpError> {
        if value.is_empty() {
            return Err(McpError::invalid_params(
                "Tool version/specification required.",
                Some(json!({
                    "param": "spec"
                })),
            ));
        }

        ToolSpec::parse(value).map_err(map_parse_error)
    }

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

    #[tool(description = "Get configuration for the current working directory.")]
    async fn get_config(&self) -> Result<CallToolResult, McpError> {
        let config = handle_tool_error!(self.session.load_config());

        Ok(CallToolResult::structured(
            serde_json::to_value(config).unwrap(),
        ))
    }

    #[tool(description = "Install a tool with a specification.")]
    async fn install_tool(
        &self,
        params: Parameters<InstallToolRequest>,
    ) -> Result<CallToolResult, McpError> {
        let req = params.0;
        let context = self.parse_context(&req.tool)?;
        let spec = self.parse_spec(req.spec.as_deref().unwrap_or("latest"))?;

        let tool = handle_tool_error!(self.session.load_tool(&context).await);
        let mut workflow = InstallWorkflow::new(tool, self.session.console.clone());

        let outcome = handle_tool_error!(
            workflow
                .install(
                    spec,
                    InstallWorkflowParams {
                        force: req.force,
                        log_writer: None,
                        multiple: false,
                        passthrough_args: vec![],
                        pin_to: if req.pin {
                            Some(PinLocation::Local)
                        } else {
                            None
                        },
                        quiet: true,
                        skip_prompts: true,
                        strategy: None,
                    },
                )
                .await
        );

        Ok(CallToolResult::structured(
            serde_json::to_value(InstallToolResponse {
                installed: matches!(
                    outcome,
                    InstallOutcome::AlreadyInstalled(_) | InstallOutcome::Installed(_)
                ),
                spec: workflow.tool.get_resolved_version().to_string(),
            })
            .unwrap(),
        ))
    }

    #[tool(description = "Uninstall a tool with a specification.")]
    async fn uninstall_tool(
        &self,
        params: Parameters<UninstallToolRequest>,
    ) -> Result<CallToolResult, McpError> {
        let req = params.0;
        let context = self.parse_context(&req.tool)?;
        let spec = self.parse_spec(&req.spec)?;

        let mut tool = handle_tool_error!(self.session.load_tool(&context).await);

        handle_tool_error!(tool.resolve_version(&spec, false).await);

        let uninstalled = handle_tool_error!(tool.uninstall().await);

        Ok(CallToolResult::structured(
            serde_json::to_value(UninstallToolResponse {
                uninstalled,
                spec: tool.get_resolved_version().to_string(),
            })
            .unwrap(),
        ))
    }

    #[tool(description = "List available and installed versions for a tool.")]
    async fn list_tool_versions(
        &self,
        params: Parameters<ListToolVersionsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let req = params.0;
        let context = self.parse_context(&req.tool)?;

        let tool = handle_tool_error!(self.session.load_tool(&context).await);

        let resolver = handle_tool_error!(
            tool.load_version_resolver(&UnresolvedVersionSpec::parse("latest").unwrap())
                .await
        );

        let versions = resolver
            .versions
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>();

        Ok(CallToolResult::structured(
            serde_json::to_value(ListToolVersionsResponse {
                aliases: resolver
                    .aliases
                    .into_iter()
                    .map(|(k, v)| (k, v.to_string()))
                    .collect(),
                installed_versions: tool
                    .inventory
                    .manifest
                    .installed_versions
                    .iter()
                    .map(|v| v.to_string())
                    .collect(),
                versions: if req.all {
                    versions
                } else {
                    versions[0..25].to_vec()
                },
            })
            .unwrap(),
        ))
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
                name: env!("CARGO_CRATE_NAME").to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                website_url: Some("https://moonrepo.dev/proto".into()),
                ..Default::default()
            },
            instructions: Some("The proto MCP server provides resources and tools for managing your toolchain, environment, and more.".to_string()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(self.list_all_resources())
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
