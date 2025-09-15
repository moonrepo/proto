use crate::utils::tool_record::ToolRecord;
use indexmap::IndexMap;
use miette::IntoDiagnostic;
use proto_core::flow::setup::ProtoSetupError;
use proto_core::{ProtoConfig, ProtoConfigEnvOptions, ToolContext, ToolSpec};
use proto_pdk_api::{
    ActivateEnvironmentInput, ActivateEnvironmentOutput, HookFunction, PluginFunction, RunHook,
    RunHookResult,
};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use tokio::task::JoinSet;

#[derive(Default)]
pub struct ExecItem {
    args: Vec<String>,
    env: IndexMap<String, Option<String>>,
    paths: Vec<PathBuf>,
}

impl ExecItem {
    pub fn add_args(&mut self, args: Vec<String>) {
        self.args.extend(args);
    }

    pub fn add_path(&mut self, path: PathBuf) {
        // Only add paths that exist
        if path.exists() {
            self.paths.push(path);
        }
    }

    pub fn set_env(&mut self, key: String, value: String) {
        self.env.insert(key, Some(value));
    }

    pub fn remove_env(&mut self, key: String) {
        self.env.insert(key, None);
    }
}

#[derive(Clone, Default)]
pub struct ExecWorkflowParams {
    pub activate_environment: bool,
    pub check_process_env: bool,
    pub detect_version: bool,
    pub passthrough_args: Vec<String>,
    pub pre_run_hook: bool,
}

pub struct ExecWorkflow<'app> {
    pub tools: Vec<ToolRecord>,
    pub env: IndexMap<String, Option<String>>,
    pub paths: VecDeque<PathBuf>,

    config: &'app ProtoConfig,
}

impl<'app> ExecWorkflow<'app> {
    pub fn new(tools: Vec<ToolRecord>, config: &'app ProtoConfig) -> Self {
        Self {
            tools,
            env: IndexMap::default(),
            paths: VecDeque::default(),
            config,
        }
    }

    pub fn collect_item(&mut self, item: ExecItem) {
        for (key, value) in item.env {
            self.env.insert(key, value);
        }

        // Don't use a set as we need to persist the order!
        for path in item.paths {
            if !self.paths.contains(&path) {
                self.paths.push_back(path);
            }
        }
    }

    pub async fn prepare_environment(
        &mut self,
        mut specs: FxHashMap<ToolContext, ToolSpec>,
        params: ExecWorkflowParams,
    ) -> miette::Result<()> {
        let mut set = JoinSet::<Result<ExecItem, ProtoSetupError>>::new();

        // Inherit shared environment variables
        self.env
            .extend(self.config.get_env_vars(ProtoConfigEnvOptions {
                include_shared: true,
                ..Default::default()
            })?);

        for tool in std::mem::take(&mut self.tools) {
            let provided_spec = specs.remove(&tool.context);
            let params = params.clone();

            // Inherit tool environment variables
            self.env
                .extend(self.config.get_env_vars(ProtoConfigEnvOptions {
                    context: Some(&tool.context),
                    check_process: params.check_process_env,
                    ..Default::default()
                })?);

            // Extract the version in a background thread
            set.spawn(async move { prepare_tool(tool, provided_spec, params).await });
        }

        while let Some(item) = set.join_next().await {
            self.collect_item(item.into_diagnostic()??);
        }

        Ok(())
    }
}

async fn prepare_tool(
    mut tool: ToolRecord,
    provided_spec: Option<ToolSpec>,
    params: ExecWorkflowParams,
) -> Result<ExecItem, ProtoSetupError> {
    let mut item = ExecItem::default();

    // Detect a version, otherwise return early
    let spec = match provided_spec {
        Some(inner) => inner,
        None => {
            if params.detect_version {
                match tool.detect_version().await {
                    Ok(inner) => inner,
                    Err(_) => {
                        return Ok(item);
                    }
                }
            } else {
                return Ok(item);
            }
        }
    };

    // Resolve the version and locate executables
    if !tool.is_setup(&spec).await? {
        return Ok(item);
    }

    // Extract vars/paths for environment
    if params.activate_environment
        && tool
            .plugin
            .has_func(PluginFunction::ActivateEnvironment)
            .await
    {
        let output: ActivateEnvironmentOutput = tool
            .plugin
            .call_func_with(
                PluginFunction::ActivateEnvironment,
                ActivateEnvironmentInput {
                    context: tool.create_plugin_context(),
                },
            )
            .await?;

        for (key, value) in output.env {
            item.set_env(key, value);
        }

        for path in output.paths {
            item.add_path(path);
        }
    }

    if params.pre_run_hook && tool.plugin.has_func(HookFunction::PreRun).await {
        let globals_dir = tool.locate_globals_dir().await?;
        let globals_prefix = tool.locate_globals_prefix().await?;

        let output: RunHookResult = tool
            .plugin
            .call_func_with(
                HookFunction::PreRun,
                RunHook {
                    context: tool.create_plugin_context(),
                    globals_dir: globals_dir.map(|dir| tool.to_virtual_path(&dir)),
                    globals_prefix,
                    passthrough_args: params.passthrough_args,
                },
            )
            .await?;

        if let Some(value) = output.args {
            item.add_args(value);
        }

        if let Some(env) = output.env {
            for (key, value) in env {
                item.set_env(key, value);
            }
        }

        if let Some(paths) = output.paths {
            for path in paths {
                item.add_path(path);
            }
        }
    }

    // Extract executable directories
    for exes_dir in tool.locate_exes_dirs().await? {
        item.add_path(exes_dir);
    }

    for globals_dir in tool.locate_globals_dirs().await? {
        item.add_path(globals_dir);
    }

    // Mark it as used so that auto-clean doesn't remove it!
    if std::env::var("PROTO_SKIP_USED_AT").is_err() {
        let _ = tool.product.track_used_at();
    }

    Ok(item)
}
