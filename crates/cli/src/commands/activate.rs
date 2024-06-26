use crate::session::ProtoSession;
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::detect_version;
use starbase::AppResult;
use starbase_shell::ShellType;
use std::path::PathBuf;
use tokio::task::{self, JoinHandle};

#[derive(Args, Clone, Debug)]
pub struct ActivateArgs {
    #[arg(help = "Shell to activate for")]
    shell: Option<ShellType>,
}

#[tracing::instrument(skip_all)]
pub async fn activate(session: ProtoSession, args: ActivateArgs) -> AppResult {
    // Detect the shell that we need to activate for
    let shell = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    // Load all the tools so that we can extract directory paths
    let tools = session.load_tools().await?;
    let mut futures: Vec<JoinHandle<miette::Result<Vec<PathBuf>>>> = vec![];

    for mut tool in tools {
        futures.push(task::spawn(async move {
            let version = detect_version(&tool, None).await?;

            // This runs the resolve -> locate flow
            if tool.is_setup(&version).await? {
                tool.locate_globals_dirs().await?;

                return Ok(tool.get_globals_dirs().to_owned());
            }

            Ok(vec![])
        }));
    }

    // Aggregate our list of paths to prepend to PATH
    let mut path = vec![
        session.env.store.shims_dir.clone(),
        session.env.store.bin_dir.clone(),
    ];

    for future in futures {
        for dir in future.await.into_diagnostic()?? {
            // Don't use a set as we need to persist the order!
            if !path.contains(&dir) {
                path.push(dir);
            }
        }
    }

    // Output PATH in shell specific syntax
    println!(
        "{}",
        shell.build().format_path_export(
            &path
                .into_iter()
                .map(|dir| dir.to_string_lossy().to_string())
                .collect::<Vec<_>>()
        )
    );

    Ok(())
}
