use crate::session::ProtoSession;
use clap::Args;
use indexmap::IndexMap;
use miette::IntoDiagnostic;
use proto_core::{detect_version, Id};
use serde::Serialize;
use starbase::AppResult;
use starbase_shell::{BoxedShell, Hook, ShellType, Statement};
use starbase_utils::json;
use std::env;
use std::path::{Path, PathBuf};
use tokio::task::JoinSet;

#[derive(Default, Serialize)]
struct ActivateItem {
    pub id: Id,
    pub env: IndexMap<String, Option<String>>,
    pub paths: Vec<PathBuf>,
}

impl ActivateItem {
    pub fn add_path(&mut self, path: &Path) {
        // Only add paths that exist and are normalized
        if let Ok(path) = path.canonicalize() {
            self.paths.push(path);
        }
    }
}

#[derive(Default, Serialize)]
struct ActivateInfo {
    pub env: IndexMap<String, Option<String>>,
    pub paths: Vec<PathBuf>,
    pub tools: Vec<ActivateItem>,
}

impl ActivateInfo {
    pub fn collect(&mut self, item: ActivateItem) {
        // Don't use a set as we need to persist the order!
        for path in &item.paths {
            if !self.paths.contains(path) {
                self.paths.push(path.to_owned());
            }
        }

        self.env.extend(item.env.clone());
        self.tools.push(item);
    }

    pub fn export(self, shell: BoxedShell) -> String {
        let mut output = vec![];

        for (key, value) in &self.env {
            output.push(shell.format_env(key, value.as_deref()));
        }

        let paths = self
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

        output.join("\n")
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

    #[arg(long, help = "Include versions from global ~/.proto/.prototools")]
    include_global: bool,

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
        match shell_type.build().format_hook(Hook::OnChangeDir {
            command: match shell_type {
                // These operate on JSON
                ShellType::Nu => format!("proto activate {} --json", shell_type),
                // While these evaluate shell syntax
                _ => format!("proto activate {} --export", shell_type),
            },
            prefix: "proto".into(),
        }) {
            Ok(output) => {
                println!("{output}");
            }
            Err(_) => {
                // Do nothing? This command is typically wrapped in `eval`,
                // so these warnings would actually just trigger a syntax error.
            }
        };

        return Ok(());
    }

    // Pre-load configuration
    let manager = session.env.load_config_manager()?;
    let config = if args.include_global {
        manager.get_merged_config()?
    } else {
        manager.get_merged_config_without_global()?
    };

    // Load all the tools so that we can extract info
    let tools = session.load_tools().await?;
    let mut set = JoinSet::<miette::Result<ActivateItem>>::new();

    for mut tool in tools {
        if !config.versions.contains_key(&tool.id) {
            continue;
        }

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

            // Inherit all environment variables for the config
            let config = tool.proto.load_config()?;

            item.env.extend(config.get_env_vars(Some(&tool.id))?);
            item.id = tool.id;

            Ok(item)
        });
    }

    // Aggregate our list of shell exports
    let mut info = ActivateInfo::default();

    // Put shims first so that they can detect newer versions
    if !args.no_shim {
        info.paths.push(session.env.store.shims_dir.clone());
    }

    while let Some(item) = set.join_next().await {
        info.collect(item.into_diagnostic()??);
    }

    // Put bins last as a last resort lookup
    if !args.no_bin {
        info.paths.push(session.env.store.bin_dir.clone());
    }

    // Output/export the information for the chosen shell
    if args.export {
        println!("{}", info.export(shell_type.build()));

        return Ok(());
    }

    if args.json {
        println!("{}", json::format(&info, true)?);

        return Ok(());
    }

    Ok(())
}
