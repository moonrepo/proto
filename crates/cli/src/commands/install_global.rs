use crate::helpers::create_progress_bar;
use crate::tools::create_tool;
use miette::IntoDiagnostic;
use proto_core::ProtoError;
use starbase::SystemResult;
use starbase_styles::color;
use tokio::process::Command;
use tracing::{debug, info};

pub async fn install_global(tool_id: String, dependencies: Vec<String>) -> SystemResult {
    let tool = create_tool(&tool_id).await?;

    for dependency in dependencies {
        debug!(tool = &tool.id, dependency, "Installing global dependency");

        let Some(globals_dir) = tool.get_globals_bin_dir() else {
            debug!("Skipping as {} it does not support globals", tool.get_name());
            continue;
        };

        let mut command = Command::new(&tool.id);

        // TODO move into plugins
        match tool.id.as_ref() {
            "bun" => {
                command.args(["add", "--global"]).arg(&dependency);
            }
            "deno" => {
                command
                    .args(["install", "--allow-net", "--allow-read"])
                    .arg(&dependency);
            }
            "go" => {
                command.arg("install").arg(&dependency);
            }
            "node" | "npm" | "pnpm" | "yarn" => {
                command = Command::new("npm");
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
                    .env("PREFIX", globals_dir.parent().unwrap());
            }
            "rust" => {
                command = Command::new("cargo");
                command.arg("install").arg("--force").arg(&dependency);
            }
            _ => {
                continue;
            }
        };

        let pb = create_progress_bar(format!("Installing {} for {}", dependency, tool.get_name()));

        let output = command
            .env("PROTO_INSTALL_GLOBAL", "true")
            .output()
            .await
            .into_diagnostic()?;

        pb.finish_and_clear();

        let stderr = String::from_utf8_lossy(&output.stderr);

        debug!("[stderr] {}", stderr);
        debug!("[stdout] {}", String::from_utf8_lossy(&output.stdout));

        if !output.status.success() {
            return Err(ProtoError::Message(stderr.to_string()))?;
        }

        info!(
            "{} has been installed to {}!",
            dependency,
            color::path(globals_dir),
        );
    }

    Ok(())
}
