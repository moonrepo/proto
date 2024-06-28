use crate::session::ProtoSession;
use clap::Args;
use indexmap::IndexMap;
use miette::IntoDiagnostic;
use proto_core::{detect_version, Id};
use serde::Serialize;
use starbase::AppResult;
use starbase_shell::{OnCdHook, ShellType};
use starbase_utils::json;
use std::path::PathBuf;
use tokio::task::{self, JoinHandle};

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
    pub fn extend_paths(&mut self, paths: Vec<PathBuf>) {
        // Don't use a set as we need to persist the order!
        for path in paths {
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
            let version = detect_version(&tool, None).await?;
            let mut item = ActivateItem::default();

            // This runs the resolve -> locate flow
            if tool.is_setup(&version).await? {
                tool.locate_globals_dirs().await?;

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

        info.extend_paths(item.paths.clone());
        info.env.extend(item.env.clone());
        info.tools.push(item);
    }

    if args.json {
        println!("{}", json::format(&info, true)?);

        return Ok(());
    }

    // Output in shell specific syntax
    println!(
        "{}",
        shell
            .build()
            .format_on_cd_hook(OnCdHook {
                env: info.env.into_iter().collect(),
                paths: info
                    .paths
                    .into_iter()
                    .map(|path| path.to_string_lossy().to_string())
                    .collect(),
                prefix: "proto".into()
            })
            // Don't error since the activate is probably called in an `eval`
            // statement, which means the error won't actually be displayed!
            .unwrap_or("".into())
    );

    Ok(())
}
