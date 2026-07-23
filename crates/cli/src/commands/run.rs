use crate::commands::install::{InstallArgs, install_one};
use crate::error::ProtoCliError;
use crate::session::{ProtoSession, SessionResult};
use crate::workflows::{ExecCommandOptions, ExecWorkflow, ExecWorkflowParams};
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::flow::detect::{Detector, ProtoDetectError};
use proto_core::flow::locate::{Locator, ProtoLocateError};
use proto_core::flow::resolve::Resolver;
use proto_core::layout::ShimRegistry;
use proto_core::{
    Id, PROTO_PLUGIN_KEY, ProtoEnvironment, ProtoLoaderError, Tool, ToolContext, ToolSpec,
};
use proto_pdk_api::ExecutableConfig;
use proto_shim::{exec_command_and_replace, locate_proto_exe};
use rustc_hash::FxHashMap;
use starbase_styles::color;
use starbase_utils::{envx, path};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, instrument};

#[derive(Args, Clone, Debug)]
pub struct RunArgs {
    #[arg(required = true, help = "Tool to run")]
    context: ToolContext,

    #[arg(help = "Version specification to run")]
    spec: Option<ToolSpec>,

    #[arg(
        long,
        alias = "alt",
        help = "File name of an alternate (secondary) executable to run"
    )]
    exe: Option<String>,

    // Passthrough args (after --)
    #[arg(
        last = true,
        help = "Arguments to pass through to the underlying command"
    )]
    passthrough: Vec<String>,
}

fn should_use_global_proto(tool: &Tool) -> miette::Result<bool> {
    if tool.get_id() != PROTO_PLUGIN_KEY {
        return Ok(false);
    }

    let config = tool.proto.load_config()?;
    let proto_context = ToolContext::new(Id::raw(PROTO_PLUGIN_KEY));

    Ok(
        // No pinnned version
        !config.versions.contains_key(&proto_context)
        // Pinned but the same as the running process
        || config.versions.get(&proto_context).is_some_and(|v| v.req.to_string() == env!("CARGO_PKG_VERSION")),
    )
}

fn should_hide_auto_install_output(args: &[String]) -> bool {
    envx::bool_var("PROTO_AUTO_INSTALL_HIDE_OUTPUT")
        || args.iter().any(|arg| arg == "--version" || arg == "--help")
}

fn is_trying_to_self_upgrade(tool: &Tool, args: &[String]) -> bool {
    if tool.get_id() == PROTO_PLUGIN_KEY
        || tool.metadata.self_upgrade_commands.is_empty()
        || args.is_empty()
    {
        return false;
    }

    // Expand "self upgrade" string into ["self", "upgrade"] list
    let mut match_groups = vec![];

    for arg_string in &tool.metadata.self_upgrade_commands {
        if let Ok(arg_list) = shell_words::split(arg_string) {
            match_groups.push(arg_list);
        }
    }

    // Then match the args in sequence
    'outer: for match_list in match_groups {
        for (index, match_arg) in match_list.into_iter().enumerate() {
            if args.get(index).is_none_or(|arg| arg != &match_arg) {
                continue 'outer;
            }
        }

        return true;
    }

    false
}

async fn get_tool_executable(
    tool: &Tool,
    spec: &ToolSpec,
    alt: Option<&str>,
) -> miette::Result<ExecutableConfig> {
    let locator = Locator::new(tool, spec);

    // Run an alternate executable (via shim)
    if let Some(alt_name) = alt {
        for location in locator.locate_shims().await? {
            if location.name == alt_name {
                let Some(exe_path) = &location.config.exe_path else {
                    continue;
                };

                let alt_exe_path = locator.product_dir.join(exe_path);

                if alt_exe_path.exists() {
                    debug!(
                        exe = alt_name,
                        path = ?alt_exe_path,
                        "Received an alternate executable to run with",
                    );

                    return Ok(ExecutableConfig {
                        exe_path: Some(alt_exe_path),
                        ..location.config
                    });
                }
            }
        }

        return Err(ProtoCliError::RunMissingAltBin {
            bin: alt_name.to_owned(),
            path: locator.product_dir.clone(),
        }
        .into());
    }

    // Otherwise use the primary
    let mut config = match locator.locate_primary_exe().await? {
        Some(inner) => inner.config,
        None => {
            return Err(ProtoLocateError::NoPrimaryExecutable {
                tool: tool.get_name().into(),
            }
            .into());
        }
    };

    // We don't use `locate_exe_file` here because we need to handle
    // tools whose primary file is not executable, like JavaScript!
    config.exe_path = Some(locator.product_dir.join(config.exe_path.as_ref().unwrap()));

    Ok(config)
}

// Set when falling back to a global executable, so that we can detect
// when the "global" executable re-enters proto (an execution loop)
const FALLBACK_GUARD_VAR: &str = "PROTO_INTERNAL_RUN_FALLBACK";

fn has_fallen_back(guard: &str, id: &str) -> bool {
    guard.split(',').any(|prev| prev == id)
}

fn append_fallback(guard: &str, id: &str) -> String {
    if guard.is_empty() {
        id.to_owned()
    } else {
        format!("{guard},{id}")
    }
}

fn get_global_executable(env: &ProtoEnvironment, name: &str) -> Option<PathBuf> {
    let system_path = env::var_os("PATH")?;

    find_global_executable(env, name, &system_path)
}

fn find_global_executable(
    env: &ProtoEnvironment,
    name: &str,
    system_path: &OsStr,
) -> Option<PathBuf> {
    let exe_name = path::exe_name(name);

    // Canonicalize both sides of every comparison, otherwise symlinked
    // paths (`/var` -> `/private/var`, linked home directories, etc)
    // will never match against each other
    let canonicalize = |path: &Path| fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());

    let skip_dirs = [
        canonicalize(&env.store.bin_dir),
        canonicalize(&env.store.shims_dir),
    ];

    for path_dir in env::split_paths(system_path) {
        let canonical_dir = canonicalize(&path_dir);

        if skip_dirs.iter().any(|dir| canonical_dir.starts_with(dir)) {
            continue;
        }

        // Another proto store may exist on PATH that doesn't match the
        // current store (changed `HOME` or `PROTO_HOME`), and executing
        // one of its shims would trigger a recursive execution loop!
        if canonical_dir.ends_with(".proto/shims") || canonical_dir.ends_with(".proto/bin") {
            continue;
        }

        // Local development may have ~/.proto on PATH, so ignore!
        #[cfg(debug_assertions)]
        if path_dir.to_string_lossy().contains(".proto") {
            continue;
        }

        let path = path_dir.join(&exe_name);

        if path.exists() && path.is_file() {
            // The file itself may be a symlink into one of our stores
            if fs::canonicalize(&path)
                .is_ok_and(|target| skip_dirs.iter().any(|dir| target.starts_with(dir)))
            {
                continue;
            }

            return Some(path);
        }
    }

    None
}

// It is possible that we have a shim for the tool, but can not find the
// plugin or version. However, this tool may exist on `PATH` outside
// of proto, so try and fallback to it!
fn run_global_tool(
    session: ProtoSession,
    args: RunArgs,
    error: miette::Report,
) -> miette::Result<()> {
    if let Some(global_exe) = get_global_executable(&session.env, args.context.id.as_str()) {
        let id = args.context.id.to_string();
        let guard = env::var(FALLBACK_GUARD_VAR).unwrap_or_default();

        // If we've already fallen back for this tool once in this execution
        // chain, then the "global" executable is actually a proto shim,
        // and executing it would recurse forever!
        if has_fallen_back(&guard, &id) {
            return Err(ProtoCliError::RunFallbackLoop {
                tool: id,
                path: global_exe,
            }
            .into());
        }

        debug!(
            global_exe = ?global_exe,
            "Tool {} is currently not managed by proto but exists on PATH, falling back to the global executable",
            color::shell(args.context.id),
        );

        let mut command = Command::new(global_exe);
        command.args(args.passthrough);
        command.env(FALLBACK_GUARD_VAR, append_fallback(&guard, &id));

        return exec_command_and_replace(command).into_diagnostic();
    }

    Err(error)
}

#[instrument(skip(session))]
pub async fn run(session: ProtoSession, mut args: RunArgs) -> SessionResult {
    let mut tool = match session.load_tool(&args.context).await {
        Ok(tool) => tool,
        Err(ProtoLoaderError::UnknownTool { id }) => {
            // Check if this is a bin provided by another tool (e.g., `npx` from `npm`).
            // The shims registry contains mappings of secondary bins to their parent tools,
            // which is maintained by proto during tool installation.
            debug!(
                bin = id.as_str(),
                "Tool not found, checking shims registry for bin-to-tool mapping"
            );

            let registry = ShimRegistry::load_from(&session.env.store.shims_dir)?;
            let mut custom_context: Option<ToolContext> = None;
            let mut before_args: Vec<String> = vec![];
            let mut after_args: Vec<String> = vec![];

            // Try reading the shims registry
            if let Some(shim_entry) = registry.get(id.as_str())
                && let Some(context) = &shim_entry.context
            {
                debug!(
                    bin = id.as_str(),
                    context = context.as_str(),
                    "Found {} in shims registry, redirecting to {}",
                    id.as_str(),
                    context
                );

                custom_context = Some(context.to_owned());

                // Store before/after args from the shim entry
                before_args = shim_entry.before_args.clone();
                after_args = shim_entry.after_args.clone();
            }

            if let Some(context) = custom_context {
                // Update args to run the parent tool with this bin as an alternate executable
                args.exe = Some(id.to_string());
                args.context = context;

                // Prepend before_args and append after_args to passthrough
                let mut new_passthrough = before_args;
                new_passthrough.extend(args.passthrough.clone());
                new_passthrough.extend(after_args);
                args.passthrough = new_passthrough;

                // Load the parent tool (this will handle auto-install if enabled,
                // or show a proper error message if the tool is not installed)
                session.load_tool(&args.context).await?
            } else {
                // Not found in shims registry, fall back to global tool on PATH
                return run_global_tool(session, args, ProtoLoaderError::UnknownTool { id }.into())
                    .map(|_| None);
            }
        }
        Err(error) => {
            return if matches!(error, ProtoLoaderError::UnknownTool { .. }) {
                run_global_tool(session, args, error.into()).map(|_| None)
            } else {
                Err(error.into())
            };
        }
    };

    let mut use_global_proto = should_use_global_proto(&tool)?;

    // Avoid running the tool's native self-upgrade as it conflicts with proto
    if is_trying_to_self_upgrade(&tool, &args.passthrough) {
        return Err(ProtoCliError::RunNoSelfUpgrade {
            command: format!("proto install {} latest --pin", tool.context),
            tool: tool.get_name().to_owned(),
        }
        .into());
    }

    // Detect a version to run with
    let (mut spec, detected_source) = if use_global_proto {
        (
            args.spec
                .clone()
                .unwrap_or_else(|| ToolSpec::parse("*").unwrap()),
            None,
        )
    } else if let Some(spec) = args.spec.clone() {
        (spec, None)
    } else {
        match Detector::detect(&tool).await {
            Ok((spec, source)) => (spec, source),
            Err(error) => {
                return if matches!(error, ProtoDetectError::FailedVersionDetect { .. }) {
                    run_global_tool(session, args, error.into()).map(|_| None)
                } else {
                    Err(error.into())
                };
            }
        }
    };

    Resolver::resolve(&tool, &mut spec, true).await?;

    // Check if installed or need to install
    if tool.is_installed(&spec) {
        if tool.get_id() == PROTO_PLUGIN_KEY {
            use_global_proto = false;
        }
    } else {
        let config = tool.proto.load_config()?;
        let resolved_version = spec.get_resolved_version();

        // Auto-install the missing tool
        if config.settings.auto_install {
            let hide_output = should_hide_auto_install_output(&args.passthrough);

            if hide_output {
                session.console.set_quiet(true);
            } else {
                session.console.out.write_line(format!(
                    "Auto-install is enabled, attempting to install {} {}",
                    tool.get_name(),
                    resolved_version,
                ))?;
            }

            install_one(
                session.clone(),
                InstallArgs {
                    internal: true,
                    quiet: hide_output,
                    spec: Some(ToolSpec {
                        req: resolved_version.to_unresolved_spec(),
                        version: Some(resolved_version.clone()),
                        version_locked: None,
                        resolve_from_manifest: false,
                        resolve_from_lockfile: false,
                        update_lockfile: false,
                    }),
                    ..Default::default()
                },
                tool.context.clone(),
            )
            .await?;

            if hide_output {
                session.console.set_quiet(false);
            } else {
                session.console.out.write_line(format!(
                    "{} {} has been installed, continuing execution...",
                    tool.get_name(),
                    resolved_version,
                ))?;
            }
        }
        // If this is the proto tool running, continue instead of failing
        else if use_global_proto {
            debug!(
                "No proto version detected or located, falling back to the global proto executable!"
            );
        }
        // Otherwise fail with a not installed error
        else {
            let command = format!("proto install {} {}", tool.context, resolved_version);

            if let Some(source) = detected_source {
                return Err(ProtoCliError::RunMissingToolWithSource {
                    tool: tool.get_name().to_owned(),
                    version: spec.req.to_string(),
                    command,
                    path: source,
                }
                .into());
            }

            return Err(ProtoCliError::RunMissingTool {
                tool: tool.get_name().to_owned(),
                version: spec.req.to_string(),
                command,
            }
            .into());
        }
    }

    // Determine the executable path to execute and create command
    let exe_config = if use_global_proto {
        ExecutableConfig {
            exe_path: locate_proto_exe("proto"),
            primary: true,
            ..Default::default()
        }
    } else {
        get_tool_executable(&tool, &spec, args.exe.as_deref()).await?
    };

    // Gather tools and specs
    tool.detected_version = Some(spec);

    let tool_name = tool.get_name().to_string();
    let tools = session.load_tool_dependencies(tool).await?;
    let specs = tools
        .iter()
        .filter_map(|tool| {
            tool.detected_version
                .clone()
                .map(|spec| (tool.context.clone(), spec))
        })
        .collect::<FxHashMap<_, _>>();

    // Prepare environment
    let config = session.load_config()?;
    let mut workflow = ExecWorkflow::new(tools, config);

    workflow
        .prepare_environment(
            specs,
            ExecWorkflowParams {
                activate_environment: true,
                check_process_env: true,
                passthrough_args: args.passthrough.clone(),
                pre_run_hook: true,
                version_env_vars: !use_global_proto,
                ..Default::default()
            },
        )
        .await?;

    // Create and run command
    let command = create_command(workflow, tool_name, exe_config, args.passthrough)?;

    // Must be the last line!
    exec_command_and_replace(command)
        .into_diagnostic()
        .map(|_| None)
}

fn create_command(
    workflow: ExecWorkflow<'_>,
    tool_name: String,
    exe_config: ExecutableConfig,
    passthrough_args: Vec<String>,
) -> miette::Result<Command> {
    let exe_path = exe_config
        .exe_path
        .as_ref()
        .expect("Could not determine executable path.")
        .to_string_lossy()
        .to_string();

    let (exe, args) = if let Some(parent_exe_path) = exe_config.parent_exe_name {
        let mut args = vec![];
        args.extend(exe_config.parent_exe_args);
        args.push(exe_path);
        args.extend(passthrough_args);

        (parent_exe_path, args)
    } else {
        (exe_path, passthrough_args)
    };

    debug!(
        exe = ?exe,
        args = ?args,
        pid = std::process::id(),
        "Running {tool_name}",
    );

    let command = workflow.create_command(
        {
            let mut list = vec![exe];
            list.extend(args);
            list
        },
        None,
        ExecCommandOptions::default(),
    )?;

    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use starbase_sandbox::create_empty_sandbox;

    mod fallback_guard {
        use super::*;

        #[test]
        fn matches_exact_ids_only() {
            assert!(has_fallen_back("node", "node"));
            assert!(has_fallen_back("node,npm", "npm"));
            assert!(!has_fallen_back("", "node"));
            assert!(!has_fallen_back("node", "npm"));
            assert!(!has_fallen_back("nodejs", "node"));
        }

        #[test]
        fn appends_ids() {
            assert_eq!(append_fallback("", "node"), "node");
            assert_eq!(append_fallback("node", "npm"), "node,npm");
        }
    }

    mod find_global {
        use super::*;

        // Avoid ".proto" in the store name, as debug builds skip
        // any path containing it
        fn create_env(sandbox: &Path) -> ProtoEnvironment {
            ProtoEnvironment::from(sandbox.join("store"), sandbox.join("home")).unwrap()
        }

        fn join_dirs(dirs: &[PathBuf]) -> std::ffi::OsString {
            env::join_paths(dirs).unwrap()
        }

        #[test]
        fn finds_exe_in_normal_dir() {
            let sandbox = create_empty_sandbox();
            let env = create_env(sandbox.path());

            // Must use the platform specific file name (tool.exe on Windows),
            // as that's what the `PATH` lookup searches for
            sandbox.create_file(format!("globals/{}", path::exe_name("tool")), "");

            let result =
                find_global_executable(&env, "tool", &join_dirs(&[sandbox.path().join("globals")]));

            assert_eq!(
                result.unwrap(),
                sandbox.path().join("globals").join(path::exe_name("tool"))
            );
        }

        #[test]
        fn skips_own_store_dirs() {
            let sandbox = create_empty_sandbox();
            let env = create_env(sandbox.path());

            sandbox.create_file(format!("store/shims/{}", path::exe_name("tool")), "");
            sandbox.create_file(format!("store/bin/{}", path::exe_name("tool")), "");

            let result = find_global_executable(
                &env,
                "tool",
                &join_dirs(&[env.store.shims_dir.clone(), env.store.bin_dir.clone()]),
            );

            assert_eq!(result, None);
        }

        #[test]
        fn skips_foreign_proto_store_dirs() {
            let sandbox = create_empty_sandbox();
            let env = create_env(sandbox.path());

            sandbox.create_file(
                format!("other-home/.proto/shims/{}", path::exe_name("tool")),
                "",
            );
            sandbox.create_file(
                format!("other-home/.proto/bin/{}", path::exe_name("tool")),
                "",
            );

            let result = find_global_executable(
                &env,
                "tool",
                &join_dirs(&[
                    sandbox.path().join("other-home/.proto/shims"),
                    sandbox.path().join("other-home/.proto/bin"),
                ]),
            );

            assert_eq!(result, None);
        }

        #[cfg(unix)]
        #[test]
        fn skips_symlinked_alias_of_store_dir() {
            let sandbox = create_empty_sandbox();
            let env = create_env(sandbox.path());

            sandbox.create_file("store/shims/tool", "");

            let alias_dir = sandbox.path().join("alias");
            std::os::unix::fs::symlink(&env.store.shims_dir, &alias_dir).unwrap();

            let result = find_global_executable(&env, "tool", &join_dirs(&[alias_dir]));

            assert_eq!(result, None);
        }

        #[cfg(unix)]
        #[test]
        fn skips_exe_symlinked_into_store() {
            let sandbox = create_empty_sandbox();
            let env = create_env(sandbox.path());

            sandbox.create_file("store/shims/tool", "");
            sandbox.create_file("globals/other", "");

            std::os::unix::fs::symlink(
                env.store.shims_dir.join("tool"),
                sandbox.path().join("globals/tool"),
            )
            .unwrap();

            let result =
                find_global_executable(&env, "tool", &join_dirs(&[sandbox.path().join("globals")]));

            assert_eq!(result, None);
        }
    }
}
