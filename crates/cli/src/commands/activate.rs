use crate::session::ProtoSession;
use clap::Args;
use indexmap::IndexMap;
use miette::IntoDiagnostic;
use proto_core::{detect_version, Id};
use serde::Serialize;
use starbase::AppResult;
use starbase_shell::{Hook, ShellType};
use starbase_styles::color;
use starbase_utils::json;
use std::path::PathBuf;
use tokio::task::{self, JoinHandle};
use tracing::warn;

#[derive(Default, Serialize)]
struct ActivateItem {
    pub id: Id,
    pub env: IndexMap<String, Option<String>>,
    pub paths: Vec<PathBuf>,
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

            // This runs the resolve -> locate flow
            if tool.is_setup(&version).await? {
                tool.locate_exes_dir().await?;
                tool.locate_globals_dirs().await?;

                if let Some(exe_dir) = tool.get_exes_dir() {
                    item.paths.push(exe_dir.to_owned());
                }

                item.paths.extend(tool.get_globals_dirs().to_owned());
            }

            item.env
                .extend(tool.proto.load_config()?.get_env_vars(Some(&tool.id))?);

            item.id = tool.id;

            Ok(item)
        }));
    }

    // Aggregate our list of shell exports
    let mut info = ActivateInfo::default();

    info.paths.extend([
        session.env.store.shims_dir.clone(),
        session.env.store.bin_dir.clone(),
    ]);

    for future in futures {
        let item = future.await.into_diagnostic()??;

        info.collect(item);
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
        Err(error) => {
            warn!(
                "Failed to run {}. Perhaps remove it for the time being?",
                color::shell("proto activate")
            );
            warn!("Reason: {}", color::muted_light(error.to_string()));
        }
    };

    Ok(())
}
