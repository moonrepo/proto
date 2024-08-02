use crate::session::ProtoSession;
use clap::Args;
use indexmap::IndexMap;
use miette::IntoDiagnostic;
use proto_core::{detect_version, Id, UnresolvedVersionSpec};
use serde::Serialize;
use starbase::AppResult;
use starbase_shell::{Hook, ShellType, Statement};
use starbase_utils::json;
use std::collections::HashSet;
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

    #[arg(long, help = "Print the activate instructions in JSON format")]
    json: bool,

    #[arg(long, help = "Don't include ~/.proto/bin in path lookup")]
    no_bin: bool,

    #[arg(long, help = "Don't include ~/.proto/shims in path lookup")]
    no_shim: bool,
}

#[tracing::instrument(skip_all)]
pub async fn activate(session: ProtoSession, args: ActivateArgs) -> AppResult {
    // Detect the shell that we need to activate for
    let shell_type = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    // If not exporting data, just print the activation syntax immediately
    if !args.export && !args.json {
        return print_activation_hook(&shell_type);
    }

    // Pre-load configuration
    let config = session.env.load_config()?;

    // Load necessary tools so that we can extract info
    let tools = session
        .load_tools_with_filters(HashSet::from_iter(config.versions.keys()))
        .await?;
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
                tool.locate_exes_dir().await?;
                tool.locate_globals_dirs().await?;

                // Higher priority over globals
                if let Some(exe_dir) = tool.get_exes_dir() {
                    item.add_path(exe_dir);
                }

                for global_dir in tool.get_globals_dirs() {
                    item.add_path(global_dir);
                }
            }

            item.id = tool.id;

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
            session.env.root.to_str().map(|root| root.to_owned()),
        );
    }

    if let Some(UnresolvedVersionSpec::Semantic(version)) = config.versions.get("proto") {
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
        print_activation_exports(&shell_type, info)?;

        return Ok(());
    }

    if args.json {
        println!("{}", json::format(&info, true)?);

        return Ok(());
    }

    Ok(())
}

fn print_activation_hook(shell_type: &ShellType) -> AppResult {
    let mut command = format!("proto activate {}", shell_type);

    for arg in env::args() {
        if arg.starts_with("--") {
            command.push(' ');
            command.push_str(&arg);
        }
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

    println!(
        "{}",
        shell_type.build().format_hook(Hook::OnChangeDir {
            command,
            prefix: "proto".into(),
        })?
    );

    Ok(())
}

fn print_activation_exports(shell_type: &ShellType, info: ActivateInfo) -> AppResult {
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

    println!("{}", output.join("\n"));

    Ok(())
}
