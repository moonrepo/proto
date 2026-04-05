use crate::utils::tool_record::ToolRecord;
use indexmap::{IndexMap, IndexSet};
use miette::IntoDiagnostic;
use proto_core::flow::locate::Locator;
use proto_core::flow::manage::ProtoManageError;
use proto_core::flow::resolve::Resolver;
use proto_core::{ProtoConfig, ProtoConfigEnvOptions, ToolContext, ToolSpec};
use proto_pdk_api::{
    ActivateEnvironmentInput, ActivateEnvironmentOutput, HookFunction, PluginFunction, RunHook,
    RunHookResult,
};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_args::parse as parse_args;
use starbase_shell::BoxedShell;
use starbase_utils::envx;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::task::JoinSet;
use tracing::trace;

#[derive(Default)]
pub struct ExecItem {
    context: ToolContext,
    active: bool,
    args: Vec<String>,
    env: IndexMap<String, Option<String>>,
    paths: IndexSet<PathBuf>,
}

impl ExecItem {
    pub fn add_args(&mut self, args: Vec<String>) {
        self.args.extend(args);
    }

    pub fn add_path(&mut self, path: PathBuf) {
        // Only add paths that exist
        if path.exists() {
            self.paths.insert(path);
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
    pub fallback_any_spec: bool,
    pub passthrough_args: Vec<String>,
    pub pre_run_hook: bool,
    pub version_env_vars: bool,
}

pub struct ExecWorkflow<'app> {
    pub args: Vec<String>,
    pub env: IndexMap<String, Option<String>>,
    pub paths: IndexSet<PathBuf>,

    config: &'app ProtoConfig,
    multiple: bool,
    tools: Vec<ToolRecord>,
}

impl<'app> ExecWorkflow<'app> {
    pub fn new(tools: Vec<ToolRecord>, config: &'app ProtoConfig) -> Self {
        Self {
            multiple: tools.len() > 1,
            tools,
            args: vec![],
            env: IndexMap::default(),
            paths: IndexSet::default(),
            config,
        }
    }

    pub fn collect_item(&mut self, item: ExecItem) {
        self.args.extend(item.args);

        for (key, value) in item.env {
            self.env.insert(key, value);
        }

        for path in item.paths {
            self.paths.insert(path);
        }
    }

    pub async fn prepare_environment(
        &mut self,
        mut specs: FxHashMap<ToolContext, ToolSpec>,
        params: ExecWorkflowParams,
    ) -> miette::Result<()> {
        let mut set = JoinSet::<Result<ExecItem, ProtoManageError>>::new();

        // Extract in a background thread
        for tool in std::mem::take(&mut self.tools) {
            let spec = specs.remove(&tool.context);

            set.spawn(Box::pin(prepare_tool(tool, spec, params.clone())));
        }

        // Inherit shared environment variables
        self.env
            .extend(self.config.get_env_vars(ProtoConfigEnvOptions {
                include_shared: true,
                ..Default::default()
            })?);

        while let Some(item) = set.join_next().await {
            let item = item.into_diagnostic()??;

            if item.active {
                // Inherit tool environment variables
                self.env
                    .extend(self.config.get_env_vars(ProtoConfigEnvOptions {
                        context: Some(&item.context),
                        check_process: params.check_process_env,
                        ..Default::default()
                    })?);

                self.collect_item(item);
            }
        }

        Ok(())
    }

    pub fn wrap_command<I, A>(&self, args: I) -> OsString
    where
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let mut out = OsString::new();

        for arg in args {
            if !out.is_empty() {
                out.push(OsStr::new(" "));
            }

            out.push(arg.as_ref());
        }

        if !self.multiple && !self.args.is_empty() {
            for arg in &self.args {
                out.push(OsStr::new(" "));
                out.push(arg);
            }
        }

        out
    }

    pub fn create_command<E, I, A>(self, exe: E, args: I) -> miette::Result<Command>
    where
        E: AsRef<OsStr>,
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let mut command = Command::new(exe);
        command.args(args);

        self.apply_to_command(&mut command, false)?;

        Ok(command)
    }

    pub fn create_command_with_shell(
        self,
        shell: BoxedShell,
        command_line: OsString,
    ) -> miette::Result<Command> {
        let mut command = shell.create_wrapped_command_with(command_line);

        self.apply_to_command(&mut command, true)?;

        Ok(command)
    }

    pub fn apply_to_command(self, command: &mut Command, with_shell: bool) -> miette::Result<()> {
        if let Some(path) = self.join_paths()? {
            command.env("PATH", path);
        }

        for (key, value) in self.env {
            match value {
                Some(value) => command.env(key, value),
                None => command.env_remove(key),
            };
        }

        if !with_shell && !self.multiple && !self.args.is_empty() {
            command.args(self.args);
        }

        trace!(
            exe = ?command.get_program().to_string_lossy(),
            args = ?command.get_args().map(|arg| arg.to_string_lossy()).collect::<Vec<_>>(),
            "Created command to execute",
        );

        Ok(())
    }

    pub fn join_paths(&self) -> miette::Result<Option<OsString>> {
        if !self.paths.is_empty() {
            let mut list = self.paths.clone();
            list.extend(envx::paths());

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

        for path in envx::paths() {
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

    pub fn requires_shell(&self, args: &[String]) -> bool {
        match parse_args(args.join(" ")) {
            Ok(command_line) => command_line.is_complex_command(),
            Err(_) => true,
        }
    }
}

async fn prepare_tool(
    tool: ToolRecord,
    provided_spec: Option<ToolSpec>,
    params: ExecWorkflowParams,
) -> Result<ExecItem, ProtoManageError> {
    let mut item = ExecItem {
        context: tool.context.clone(),
        ..Default::default()
    };

    // Extract the spec, otherwise return early
    let mut spec = match provided_spec {
        Some(inner) => inner,
        None => {
            if params.fallback_any_spec {
                ToolSpec::parse("*")?
            } else {
                return Ok(item);
            }
        }
    };

    item.active = true;

    // Resolve the version and locate executables
    Resolver::resolve(&tool, &mut spec, true).await?;

    if !tool.is_installed(&spec) {
        return Ok(item);
    }

    if params.version_env_vars {
        item.set_env(
            format!("{}_VERSION", tool.get_env_var_prefix()),
            spec.get_resolved_version().to_string(),
        );
    }

    // Extract vars/paths for environment
    let locations = Locator::locate(&tool, &spec).await?;

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
                    context: tool.create_plugin_context(&spec),
                    globals_dir: locations
                        .globals_dir
                        .as_ref()
                        .map(|dir| tool.to_virtual_path(dir)),
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
        let output: RunHookResult = tool
            .plugin
            .call_func_with(
                HookFunction::PreRun,
                RunHook {
                    context: tool.create_plugin_context(&spec),
                    globals_dir: locations
                        .globals_dir
                        .as_ref()
                        .map(|dir| tool.to_virtual_path(dir)),
                    globals_prefix: locations.globals_prefix,
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
    if let Some(dir) = locations.exe_file.parent() {
        item.add_path(dir.to_path_buf());
    }

    for exes_dir in locations.exes_dirs {
        item.add_path(exes_dir);
    }

    for globals_dir in locations.globals_dirs {
        item.add_path(globals_dir);
    }

    // Mark it as used so that auto-clean doesn't remove it!
    if std::env::var("PROTO_SKIP_USED_AT").is_err()
        && let Some(version) = &spec.version
    {
        let _ = tool.inventory.create_product(version).track_used_at();
    }

    Ok(item)
}
