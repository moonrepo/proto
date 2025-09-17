use crate::utils::tool_record::ToolRecord;
use indexmap::IndexMap;
use miette::IntoDiagnostic;
use proto_core::flow::setup::ProtoSetupError;
use proto_core::{ProtoConfig, ProtoConfigEnvOptions, ToolContext, ToolSpec};
use proto_pdk_api::{
    ActivateEnvironmentInput, ActivateEnvironmentOutput, HookFunction, PluginFunction, RunHook,
    RunHookResult,
};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::env::paths;
use std::collections::VecDeque;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
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

    // pub fn remove_env(&mut self, key: String) {
    //     self.env.insert(key, None);
    // }
}

#[derive(Clone, Default)]
pub struct ExecWorkflowParams {
    pub activate_environment: bool,
    pub check_process_env: bool,
    pub detect_version: bool,
    pub passthrough_args: Vec<String>,
    pub pre_run_hook: bool,
    pub version_env_vars: bool,
}

pub struct ExecWorkflow<'app> {
    pub args: Vec<String>,
    pub env: IndexMap<String, Option<String>>,
    pub paths: VecDeque<PathBuf>,

    config: &'app ProtoConfig,
    tools: Vec<ToolRecord>,
}

impl<'app> ExecWorkflow<'app> {
    pub fn new(tools: Vec<ToolRecord>, config: &'app ProtoConfig) -> Self {
        Self {
            tools,
            args: vec![],
            env: IndexMap::default(),
            paths: VecDeque::default(),
            config,
        }
    }

    pub fn collect_item(&mut self, item: ExecItem) {
        self.args.extend(item.args);

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

    pub fn apply_to_command(self, command: &mut Command) -> miette::Result<()> {
        if let Some(path) = self.join_paths()? {
            command.env("PATH", path);
        }

        for (key, value) in self.env {
            match value {
                Some(value) => command.env(key, value),
                None => command.env_remove(key),
            };
        }

        if !self.args.is_empty() {
            command.args(self.args);
        }

        Ok(())
    }

    pub fn join_paths(&self) -> miette::Result<Option<OsString>> {
        if !self.paths.is_empty() {
            let mut list = self.paths.clone().into_iter().collect::<Vec<_>>();
            list.extend(paths());

            return Ok(Some(env::join_paths(list).into_diagnostic()?));
        }

        Ok(None)
    }

    pub fn reset_paths(&self, store_dir: &Path) -> Vec<PathBuf> {
        let start_path = store_dir.join("activate-start");
        let stop_path = store_dir.join("activate-stop");

        // Create a new `PATH` list with our activated tools. Use fake
        // marker paths to indicate a boundary.
        let mut reset_paths = vec![];
        reset_paths.push(start_path.clone());
        reset_paths.extend(self.paths.clone());
        reset_paths.push(stop_path.clone());

        // `PATH` may have already been activated, so we need to remove
        // paths that proto has injected, otherwise this paths list
        // will continue to grow and grow.
        let mut in_activate = false;
        let mut dupe_paths = FxHashSet::from_iter(reset_paths.clone());

        for path in paths() {
            if path == start_path {
                in_activate = true;
                continue;
            } else if path == stop_path {
                in_activate = false;
                continue;
            } else if in_activate || dupe_paths.contains(&path) {
                continue;
            }

            reset_paths.push(path.clone());
            dupe_paths.insert(path);
        }

        reset_paths
    }

    pub fn reset_and_join_paths(&self, store_dir: &Path) -> miette::Result<OsString> {
        env::join_paths(self.reset_paths(store_dir)).into_diagnostic()
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

    if params.version_env_vars {
        item.set_env(
            format!("{}_VERSION", tool.get_env_var_prefix()),
            tool.get_resolved_version().to_string(),
        );
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
