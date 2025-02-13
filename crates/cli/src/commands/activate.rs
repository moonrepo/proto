use crate::session::ProtoSession;
use clap::Args;
use indexmap::IndexMap;
use miette::IntoDiagnostic;
use proto_core::{detect_version, Id, UnresolvedVersionSpec};
use serde::Serialize;
use starbase::AppResult;
use starbase_shell::{Hook, ShellType, Statement};
use starbase_utils::json;
use std::env;
use std::path::{Path, PathBuf};
use tokio::task::JoinSet;

#[derive(Default, Serialize)]
struct ActivateItem {
    pub id: Id,
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
struct ActivateInfo {
    pub env: IndexMap<String, Option<String>>,
    pub paths: Vec<PathBuf>,
}

impl ActivateInfo {
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
        return print_activation_hook(&session, &shell_type, &args);
    }

    // Pre-load configuration
    let config = session.env.load_config()?;

    // Load necessary tools so that we can extract info
    let tools = session.load_tools().await?;
    let mut info = ActivateInfo::default();
    let mut set = JoinSet::<miette::Result<ActivateItem>>::new();

    for mut tool in tools {
        if !config.versions.contains_key(&tool.id) {
            continue;
        }

        // Inherit all environment variables for the config
        info.env.extend(config.get_env_vars(Some(&tool.id))?);

        // Extract the version in a background thread
        set.spawn(async move {
            let mut item = ActivateItem::default();

            // Detect a version, otherwise return early
            let Ok(version) = detect_version(&tool, None).await else {
                return Ok(item);
            };

            // Resolve the version and locate executables
            if tool.is_setup(&version).await? {
                // Higher priority over globals
                if let Some(exes_dir) = tool.locate_exes_dir().await? {
                    item.add_path(&exes_dir);
                }

                for globals_dir in tool.locate_globals_dirs().await? {
                    item.add_path(&globals_dir);
                }
            }

            item.id = tool.id.clone();

            Ok(item)
        });
    }

    // Put shims first so that they can detect newer versions
    if !args.no_shim {
        info.paths.push(session.env.store.shims_dir.clone());
    }

    // Aggregate our list of shell exports
    while let Some(item) = set.join_next().await {
        info.collect(item.into_diagnostic()??);
    }

    // Inject necessary variables
    if !info.env.contains_key("PROTO_HOME") && env::var("PROTO_HOME").is_err() {
        info.env.insert(
            "PROTO_HOME".into(),
            session.env.store.dir.to_str().map(|root| root.to_owned()),
        );
    }

    if let Some(UnresolvedVersionSpec::Semantic(version)) =
        config.versions.get("proto").map(|spec| &spec.req)
    {
        info.env
            .insert("PROTO_VERSION".into(), Some(version.to_string()));

        info.paths.push(
            session
                .env
                .store
                .inventory_dir
                .join("proto")
                .join(version.to_string()),
        );
    } else {
        info.env.insert("PROTO_VERSION".into(), None);
    }

    // Put bins last as a last resort lookup
    if !args.no_bin {
        info.paths.push(session.env.store.bin_dir.clone());
    }

    // Output/export the information for the chosen shell
    if args.export {
        print_activation_exports(&session, &shell_type, info)?;

        return Ok(None);
    }

    if session.should_print_json() {
        session.console.out.write_line(json::format(&info, true)?)?;

        return Ok(None);
    }

    Ok(None)
}

fn print_activation_hook(
    session: &ProtoSession,
    shell_type: &ShellType,
    args: &ActivateArgs,
) -> AppResult {
    let mut command = format!("proto activate {}", shell_type);

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

    session
        .console
        .out
        .write_line(shell_type.build().format_hook(Hook::OnChangeDir {
            command,
            function: "_proto_activate_hook".into(),
        })?)?;

    if args.on_init {
        session.console.out.write_line("\n_proto_activate_hook")?;
    }

    Ok(None)
}

fn print_activation_exports(
    session: &ProtoSession,
    shell_type: &ShellType,
    info: ActivateInfo,
) -> AppResult {
    let shell = shell_type.build();
    let mut output = vec![];

    for (key, value) in &info.env {
        output.push(shell.format_env(key, value.as_deref()));
    }

    let paths = info
        .paths
        .iter()
        .filter_map(|path| path.to_str().map(|p| p.to_owned()))
        .collect::<Vec<_>>();

    if !paths.is_empty() {
        output.push(shell.format(Statement::PrependPath {
            paths: &paths,
            key: Some("PATH"),
            orig_key: if env::var("__ORIG_PATH").is_ok() {
                Some("__ORIG_PATH")
            } else {
                None
            },
        }));
    }

    session.console.out.write_line(output.join("\n"))?;

    Ok(None)
}
