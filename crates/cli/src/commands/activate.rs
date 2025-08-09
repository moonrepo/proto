use crate::session::ProtoSession;
use clap::Args;
use indexmap::IndexMap;
use miette::IntoDiagnostic;
use proto_core::flow::setup::ProtoSetupError;
use proto_core::{Id, PROTO_PLUGIN_KEY, ProtoConfigEnvOptions, ToolContext, UnresolvedVersionSpec};
use rustc_hash::FxHashSet;
use serde::Serialize;
use starbase::AppResult;
use starbase_shell::{Hook, ShellType};
use starbase_utils::env::paths;
use starbase_utils::json;
use std::env;
use std::path::{Path, PathBuf};
use tokio::task::JoinSet;
use tracing::warn;

#[derive(Default, Serialize)]
struct ActivateItem {
    pub tool: ToolContext,
    pub paths: Vec<PathBuf>,
}

impl ActivateItem {
    pub fn add_path(&mut self, path: &Path) {
        // Only add paths that exist
        if path.exists() {
            self.paths.push(path.to_owned());
        }
    }
}

#[derive(Default, Serialize)]
struct ActivateCollection {
    pub env: IndexMap<String, Option<String>>,
    pub path: Option<String>,
    #[serde(skip)]
    pub paths: Vec<PathBuf>,
}

impl ActivateCollection {
    pub fn collect(&mut self, item: ActivateItem) {
        // Don't use a set as we need to persist the order!
        for path in item.paths {
            if !self.paths.contains(&path) {
                self.paths.push(path);
            }
        }
    }
}

#[derive(Args, Clone, Debug)]
pub struct ActivateArgs {
    #[arg(help = "Shell to activate for")]
    shell: Option<ShellType>,

    #[arg(
        long,
        help = "Print the activate instructions in shell specific-syntax"
    )]
    export: bool,

    #[arg(long, help = "Don't include ~/.proto/bin in path lookup")]
    no_bin: bool,

    #[arg(long, help = "Do not run activate hook on initialization")]
    no_init: bool,

    #[arg(long, help = "Don't include ~/.proto/shims in path lookup")]
    no_shim: bool,

    #[arg(long, help = "Run activate hook on initialization and export")]
    on_init: bool,
}

#[tracing::instrument(skip_all)]
pub async fn activate(session: ProtoSession, args: ActivateArgs) -> AppResult {
    // Detect the shell that we need to activate for
    let shell_type = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    // If not exporting data, just print the activation syntax immediately
    if !args.export && !session.should_print_json() {
        print_activation_hook(&session, &shell_type, &args)?;

        return Ok(None);
    }

    // Pre-load configuration
    let config = session.env.load_config()?;

    // Load necessary tools so that we can extract info
    let tools = session.load_tools().await?;
    let mut collection = ActivateCollection::default();
    let mut set = JoinSet::<Result<ActivateItem, ProtoSetupError>>::new();

    // Inherit shared environment variables
    collection
        .env
        .extend(config.get_env_vars(ProtoConfigEnvOptions {
            include_shared: true,
            ..Default::default()
        })?);

    for mut tool in tools {
        if !config.versions.contains_key(&tool.context) {
            continue;
        }

        // Inherit tool environment variables
        collection
            .env
            .extend(config.get_env_vars(ProtoConfigEnvOptions {
                tool_id: Some(tool.get_id().clone()),
                ..Default::default()
            })?);

        // Extract the version in a background thread
        set.spawn(async move {
            let mut item = ActivateItem::default();

            // Detect a version, otherwise return early
            let Ok(spec) = tool.detect_version().await else {
                return Ok(item);
            };

            // Resolve the version and locate executables
            if tool.is_setup(&spec).await? {
                // Higher priority over globals
                for exes_dir in tool.locate_exes_dirs().await? {
                    item.add_path(&exes_dir);
                }

                for globals_dir in tool.locate_globals_dirs().await? {
                    item.add_path(&globals_dir);
                }

                // Mark it as used so that auto-clean doesn't remove it!
                tool.product.track_used_at()?;
            }

            item.tool = tool.context.clone();

            Ok(item)
        });
    }

    // Put shims first so that they can detect newer versions
    if !args.no_shim {
        collection.paths.push(session.env.store.shims_dir.clone());
    }

    // Aggregate our list of shell exports
    while let Some(item) = set.join_next().await {
        collection.collect(item.into_diagnostic()??);
    }

    // Inject necessary variables
    if !collection.env.contains_key("PROTO_HOME") && env::var("PROTO_HOME").is_err() {
        collection.env.insert(
            "PROTO_HOME".into(),
            session.env.store.dir.to_str().map(|root| root.to_owned()),
        );
    }

    let proto_context = ToolContext::new(Id::raw(PROTO_PLUGIN_KEY));

    if let Some(UnresolvedVersionSpec::Semantic(version)) =
        config.versions.get(&proto_context).map(|spec| &spec.req)
    {
        collection
            .env
            .insert("PROTO_VERSION".into(), Some(version.to_string()));
        collection
            .env
            .insert("PROTO_PROTO_VERSION".into(), Some(version.to_string()));

        collection.paths.push(
            session
                .env
                .store
                .inventory_dir
                .join("proto")
                .join(version.to_string()),
        );
    } else {
        collection.env.insert("PROTO_VERSION".into(), None);
    }

    // Put bins last as a last resort lookup
    if !args.no_bin {
        collection.paths.push(session.env.store.bin_dir.clone());
    }

    // Output/export the information for the chosen shell
    if args.export {
        print_activation_exports(&session, &shell_type, collection)?;

        return Ok(None);
    }

    if session.should_print_json() {
        collection.path = env::join_paths(reset_and_join_paths(
            &session,
            std::mem::take(&mut collection.paths),
        ))
        .into_diagnostic()?
        .into_string()
        .ok();

        session
            .console
            .out
            .write_line(json::format(&collection, true)?)?;
    }

    Ok(None)
}

fn print_activation_hook(
    session: &ProtoSession,
    shell_type: &ShellType,
    args: &ActivateArgs,
) -> miette::Result<()> {
    let mut command = format!("proto activate {shell_type}");

    if let Some(mode) = &session.cli.config_mode {
        command.push_str(" --config-mode ");
        command.push_str(&mode.to_string());
    }

    if args.no_bin {
        command.push_str(" --no-bin");
    }

    if args.no_shim {
        command.push_str(" --no-shim");
    }

    match shell_type {
        // These operate on JSON
        ShellType::Nu => {
            command.push_str(" --json");
        }
        // While these evaluate shell syntax
        _ => {
            command.push_str(" --export");
        }
    };

    if args.on_init {
        warn!(
            "The --on-init option is deprecated and can be removed. This functionality is now the default."
        );
    }

    session
        .console
        .out
        .write_line(shell_type.build().format_hook(Hook::OnChangeDir {
            command,
            function: "_proto_activate_hook".into(),
        })?)?;

    if !args.no_init {
        session.console.out.write_line("\n_proto_activate_hook")?;
    }

    Ok(())
}

fn print_activation_exports(
    session: &ProtoSession,
    shell_type: &ShellType,
    info: ActivateCollection,
) -> miette::Result<()> {
    let shell = shell_type.build();
    let mut env_being_set = vec![];
    let mut output = vec![];

    // Remove previously set variables
    if let Ok(env_to_remove) = env::var("_PROTO_ACTIVATED_ENV") {
        for key in env_to_remove.split(',') {
            if !info.env.contains_key(key) {
                output.push(shell.format_env_unset(key));
            }
        }
    }

    // Set/remove new variables
    for (key, value) in info.env {
        if value.is_some() {
            env_being_set.push(key.clone());
        }

        output.push(shell.format_env(&key, value.as_deref()));
    }

    if !env_being_set.is_empty() {
        output.push(shell.format_env_set("_PROTO_ACTIVATED_ENV", &env_being_set.join(",")));
    }

    // Set new `PATH`
    if !info.paths.is_empty() {
        output.push(
            shell.format_env_set(
                "_PROTO_ACTIVATED_PATH",
                &info
                    .paths
                    .iter()
                    .flat_map(|p| p.to_str())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        );

        let paths = reset_and_join_paths(session, info.paths);

        if !paths.is_empty() {
            output.push(shell.format_path_set(&paths));
        }
    }

    session.console.out.write_line(output.join("\n"))?;

    Ok(())
}

fn reset_and_join_paths(session: &ProtoSession, join_paths: Vec<PathBuf>) -> Vec<String> {
    let start_path = session.env.store.dir.join("activate-start");
    let stop_path = session.env.store.dir.join("activate-stop");

    // Create a new `PATH` list with our activated tools. Use fake
    // marker paths to indicate a boundary.
    let mut reset_paths = vec![];
    reset_paths.push(start_path.clone());
    reset_paths.extend(join_paths);
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
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}
