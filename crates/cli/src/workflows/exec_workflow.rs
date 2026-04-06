use crate::utils::tool_record::{ToolRecord, sort_tools_by_dependency};
use futures::StreamExt;
use futures::stream::FuturesOrdered;
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
use std::collections::VecDeque;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;
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
    pub paths: VecDeque<PathBuf>,

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
            paths: VecDeque::default(),
            config,
        }
    }

    pub fn collect_item(&mut self, mut item: ExecItem) {
        self.args.extend(item.args);

        for (key, value) in item.env {
            self.env.insert(key, value);
        }

        // Tools run in dependency order, so latter paths must take precedence
        // over former paths, and in regards to `PATH`, that means the paths must
        // come before, so we push to the front. Additionally, since this is pushing
        // in reverse order, we need to reverse the paths first to ensure the
        // original order is preserved.
        item.paths.reverse();

        for path in item.paths {
            self.paths.push_front(path);
        }
    }

    pub async fn prepare_environment(
        &mut self,
        mut specs: FxHashMap<ToolContext, ToolSpec>,
        params: ExecWorkflowParams,
    ) -> miette::Result<()> {
        let mut futures = FuturesOrdered::<_>::new();

        // Extract in a background thread
        for tool in sort_tools_by_dependency(std::mem::take(&mut self.tools))? {
            let spec = specs.remove(&tool.context);

            futures.push_back(tokio::spawn(Box::pin(prepare_tool(
                tool,
                spec,
                params.clone(),
            ))));
        }

        // Inherit shared environment variables
        self.env
            .extend(self.config.get_env_vars(&ProtoConfigEnvOptions {
                include_shared: true,
                ..Default::default()
            })?);

        while let Some(item) = futures.next().await {
            let item = item.into_diagnostic()??;

            if item.active {
                // Inherit tool environment variables
                self.env
                    .extend(self.config.get_env_vars(&ProtoConfigEnvOptions {
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
            let env_paths = envx::paths();
            let joined = env::join_paths(self.paths.iter().chain(&env_paths)).into_diagnostic()?;

            return Ok(Some(joined));
        }

        Ok(None)
    }

    pub fn reset_paths(&self, store_dir: &Path) -> Vec<PathBuf> {
        let start_path = store_dir.join("activate-start");
        let stop_path = store_dir.join("activate-stop");

        // Create a new `PATH` list with our activated tools. Use fake
        // marker paths to indicate a boundary.
        let mut reset_paths = Vec::with_capacity(2 + self.paths.len());
        reset_paths.push(start_path.clone());
        reset_paths.extend(self.paths.iter().cloned());
        reset_paths.push(stop_path.clone());

        // `PATH` may have already been activated, so we need to remove
        // paths that proto has injected, otherwise this paths list
        // will continue to grow and grow.
        let mut in_activate = false;
        let mut dupe_paths: FxHashSet<PathBuf> = reset_paths.iter().cloned().collect();

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

#[cfg(test)]
mod tests {
    use super::*;
    use proto_core::ProtoConfig;

    fn make_item(paths: Vec<PathBuf>, env: Vec<(&str, &str)>) -> ExecItem {
        let mut item = ExecItem {
            active: true,
            paths: paths.into_iter().collect(),
            ..Default::default()
        };

        for (k, v) in env {
            item.set_env(k.to_string(), v.to_string());
        }

        item
    }

    mod exec_item_tests {
        use super::*;

        #[test]
        fn add_path_only_adds_existing() {
            let mut item = ExecItem::default();
            item.add_path(std::env::temp_dir());
            item.add_path(PathBuf::from("/nonexistent_proto_test_xyz_12345"));

            assert_eq!(item.paths.len(), 1);
            assert!(item.paths.contains(&std::env::temp_dir()));
        }

        #[test]
        fn add_path_deduplicates() {
            let mut item = ExecItem::default();
            let tmp = std::env::temp_dir();
            item.add_path(tmp.clone());
            item.add_path(tmp.clone());

            assert_eq!(item.paths.len(), 1);
        }

        #[test]
        fn add_path_preserves_insertion_order() {
            let mut item = ExecItem::default();
            // Use two paths that definitely exist
            let tmp = std::env::temp_dir();
            let root = if cfg!(windows) {
                PathBuf::from("C:\\")
            } else {
                PathBuf::from("/")
            };
            item.add_path(tmp.clone());
            item.add_path(root.clone());

            let paths: Vec<_> = item.paths.into_iter().collect();
            assert_eq!(paths, vec![tmp, root]);
        }
    }

    mod exec_workflow_tests {
        use super::*;

        fn make_workflow() -> ExecWorkflow<'static> {
            // Use a leaked static ref for the config to satisfy the lifetime
            let config: &'static ProtoConfig = Box::leak(Box::new(ProtoConfig::default()));
            ExecWorkflow::new(vec![], config)
        }

        #[test]
        fn collect_item_preserves_path_order() {
            let mut wf = make_workflow();

            let paths = vec![
                PathBuf::from("/first"),
                PathBuf::from("/second"),
                PathBuf::from("/third"),
            ];
            wf.collect_item(make_item(paths.clone(), vec![]));

            let result: Vec<_> = wf.paths.iter().collect();
            assert_eq!(result, paths.iter().collect::<Vec<_>>());
        }

        #[test]
        fn collect_item_env_later_overrides_earlier() {
            let mut wf = make_workflow();

            wf.collect_item(make_item(vec![], vec![("KEY", "first")]));
            wf.collect_item(make_item(vec![], vec![("KEY", "second")]));

            assert_eq!(wf.env.get("KEY"), Some(&Some("second".to_string())));
        }

        #[test]
        fn join_paths_returns_none_when_empty() {
            let wf = make_workflow();
            assert!(wf.join_paths().unwrap().is_none());
        }

        #[test]
        fn join_paths_returns_some_when_non_empty() {
            let mut wf = make_workflow();
            wf.paths.push_back(PathBuf::from("/test/bin"));

            let result = wf.join_paths().unwrap();
            assert!(result.is_some());

            let joined = result.unwrap().to_string_lossy().to_string();
            assert!(joined.starts_with("/test/bin"));
        }
    }
}
