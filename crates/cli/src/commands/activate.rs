use crate::session::ProtoSession;
use clap::Args;
use indexmap::IndexMap;
use miette::IntoDiagnostic;
use proto_core::{detect_version, Id};
use serde::Serialize;
use starbase::AppResult;
use starbase_shell::{Hook, ShellType};
use starbase_utils::json;
use std::path::{Path, PathBuf};
use tokio::task::{self, JoinHandle};

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
}

#[derive(Args, Clone, Debug)]
pub struct ActivateArgs {
    #[arg(help = "Shell to activate for")]
    shell: Option<ShellType>,

    #[arg(long, help = "Print the info in JSON format")]
    json: bool,

    #[arg(long, help = "Don't include ~/.proto/bin in path lookup")]
    no_bin: bool,

    #[arg(long, help = "Don't include ~/.proto/shims in path lookup")]
    no_shim: bool,
}

#[tracing::instrument(skip_all)]
pub async fn activate(session: ProtoSession, args: ActivateArgs) -> AppResult {
    // Detect the shell that we need to activate for
    let shell = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    // Pre-load configuration
    session.env.load_config()?;

    // Load all the tools so that we can extract info
    let tools = session.load_tools().await?;
    let mut futures: Vec<JoinHandle<miette::Result<ActivateItem>>> = vec![];

    for mut tool in tools {
        futures.push(task::spawn(async move {
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
        }));
    }

    // Aggregate our list of shell exports
    let mut info = ActivateInfo::default();

    // Put shims first so that they can detect newer versions
    if !args.no_shim {
        info.paths.push(session.env.store.shims_dir.clone());
    }

    for future in futures {
        info.collect(future.await.into_diagnostic()??);
    }

    // Put bins last as a last resort lookup
    if !args.no_bin {
        info.paths.push(session.env.store.bin_dir.clone());
    }

    if args.json {
        println!("{}", json::format(&info, true)?);

        return Ok(());
    }

    // Output in shell specific syntax
    match shell.build().format_hook(Hook::OnChangeDir {
        env: info.env.into_iter().collect(),
        paths: info
            .paths
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
        prefix: "proto".into(),
    }) {
        Ok(output) => {
            println!("{output}");
        }
        Err(_) => {
            // Do nothing? This command is typically wrapped in `eval`,
            // so these warnings would actually just trigger a syntax error.

            // warn!(
            //     "Failed to run {}. Perhaps remove it for the time being?",
            //     color::shell("proto activate")
            // );
            // warn!("Reason: {}", color::muted_light(error.to_string()));
        }
    };

    Ok(())
}
