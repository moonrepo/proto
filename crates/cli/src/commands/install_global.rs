use crate::helpers::{create_progress_bar, enable_logging};
use crate::tools::{create_tool, ToolType};
use log::{debug, info, trace};
use proto::{get_home_dir, get_tools_dir, Tool};
use proto_core::{color, ProtoError};
use std::env;
use std::path::PathBuf;
use tokio::process::Command;

async fn get_bin_or_fallback(mut tool: Box<dyn Tool<'_>>) -> Result<PathBuf, ProtoError> {
    Ok(match tool.find_bin_path().await {
        Ok(_) => tool.get_bin_path()?.to_path_buf(),
        Err(_) => PathBuf::from(tool.get_bin_name()),
    })
}

pub async fn install_global(tool_type: ToolType, package: String) -> Result<(), ProtoError> {
    enable_logging();

    let tool = create_tool(&tool_type)?;
    let global_dir;
    let mut command;

    debug!(
        target: "proto:install-global",
        "Installing \"{}\" for {}",
        package,
        tool.get_name(),
    );

    match tool_type {
        ToolType::Bun => {
            global_dir = get_home_dir()?.join("bun");

            command = Command::new(get_bin_or_fallback(tool).await?);
            command.args(["add", "--global"]).arg(&package);
        }

        ToolType::Deno => {
            global_dir = match env::var("DENO_INSTALL_ROOT") {
                Ok(path) => PathBuf::from(path),
                Err(_) => get_home_dir()?.join(".deno"),
            };

            command = Command::new(get_bin_or_fallback(tool).await?);
            command
                .args(["install", "--allow-net", "--allow-read"])
                .arg(&package);
        }

        ToolType::Go => {
            global_dir = match env::var("GOBIN").or_else(|_| env::var("GOPATH")) {
                Ok(path) => PathBuf::from(path),
                Err(_) => get_home_dir()?.join("go"),
            };

            command = Command::new(get_bin_or_fallback(tool).await?);
            command.arg("install").arg(&package);
        }

        ToolType::Node | ToolType::Npm | ToolType::Pnpm | ToolType::Yarn => {
            global_dir = get_tools_dir()?.join("node").join("globals");

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
                .arg(&package)
                .env("PREFIX", &global_dir);
        }

        ToolType::Rust => {
            global_dir = match env::var("CARGO_INSTALL_ROOT") {
                Ok(path) => PathBuf::from(path),
                Err(_) => get_home_dir()?.join(".cargo"),
            };

            command = Command::new("cargo");
            command.arg("install").arg("--force").arg(&package);
        }
    };

    let pb = create_progress_bar(format!("Installing {}", package));

    let output = command
        .output()
        .await
        .map_err(|e| ProtoError::Message(e.to_string()))?;

    pb.finish_and_clear();

    trace!(
        target: "proto:install-global",
        "[stdout] {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    trace!(
        target: "proto:install-global",
        "[stderr] {}",
        stderr
    );

    if !output.status.success() {
        return Err(ProtoError::Message(stderr.to_string()));
    }

    info!(
        target: "proto:install-global", "{} has been installed at {}!",
        package,
        color::path(global_dir),
    );

    Ok(())
}
