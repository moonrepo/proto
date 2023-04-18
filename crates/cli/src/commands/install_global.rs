use crate::helpers::create_progress_bar;
use crate::tools::{create_tool, ToolType};
use proto_core::{color, ProtoError, Tool};
use starbase::SystemResult;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, info};

async fn get_bin_or_fallback(mut tool: Box<dyn Tool<'_>>) -> Result<PathBuf, ProtoError> {
    Ok(match tool.find_bin_path().await {
        Ok(_) => tool.get_bin_path()?.to_path_buf(),
        Err(_) => PathBuf::from(tool.get_bin_name()),
    })
}

pub async fn install_global(tool_type: ToolType, dependencies: Vec<String>) -> SystemResult {
    for dependency in dependencies {
        let tool = create_tool(&tool_type)?;
        let label = format!("Installing {} for {}", dependency, tool.get_name());
        let global_dir = tool.get_globals_bin_dir()?;
        let mut command;

        debug!("{}", label);

        match tool_type {
            ToolType::Bun => {
                command = Command::new(get_bin_or_fallback(tool).await?);
                command.args(["add", "--global"]).arg(&dependency);
            }

            ToolType::Deno => {
                command = Command::new(get_bin_or_fallback(tool).await?);
                command
                    .args(["install", "--allow-net", "--allow-read"])
                    .arg(&dependency);
            }

            ToolType::Go => {
                command = Command::new(get_bin_or_fallback(tool).await?);
                command.arg("install").arg(&dependency);
            }

            ToolType::Node | ToolType::Npm | ToolType::Pnpm | ToolType::Yarn => {
                let npm = create_tool(&ToolType::Npm)?;

                command = Command::new(get_bin_or_fallback(npm).await?);
                command
                    .args([
                        "install",
                        "--global",
                        "--loglevel",
                        "warn",
                        "--no-audit",
                        "--no-update-notifier",
                    ])
                    .arg(&dependency)
                    // Remove the /bin component
                    .env("PREFIX", global_dir.parent().unwrap());
            }

            ToolType::Rust => {
                command = Command::new("cargo");
                command.arg("install").arg("--force").arg(&dependency);
            }
        };

        let pb = create_progress_bar(label);

        let output = command
            .output()
            .await
            .map_err(|e| ProtoError::Message(e.to_string()))?;

        pb.finish_and_clear();

        let stderr = String::from_utf8_lossy(&output.stderr);

        debug!("[stderr] {}", stderr);
        debug!("[stdout] {}", String::from_utf8_lossy(&output.stdout));

        if !output.status.success() {
            return Err(ProtoError::Message(stderr.to_string()))?;
        }

        info!(
            "{} has been installed at {}!",
            dependency,
            color::path(global_dir),
        );
    }

    Ok(())
}
