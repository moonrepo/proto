use crate::helpers::create_progress_bar;
use crate::tools::{create_tool, ToolType};
use proto::Describable;
use proto_core::{color, ProtoError, Tool};
use proto_schema_plugin::SchemaPlugin;
use starbase::SystemResult;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, info};

async fn get_bin_or_fallback(tool: &mut Box<dyn Tool<'_>>) -> Result<PathBuf, ProtoError> {
    Ok(match tool.find_bin_path().await {
        Ok(_) => tool.get_bin_path()?.to_path_buf(),
        Err(_) => PathBuf::from(tool.get_id()),
    })
}

pub async fn install_global(tool_type: ToolType, dependencies: Vec<String>) -> SystemResult {
    for dependency in dependencies {
        let mut tool = create_tool(&tool_type).await?;
        let label = format!("Installing {} for {}", dependency, tool.get_name());

        debug!("{}", label);

        let Some(global_dir) = tool.get_globals_bin_dir()? else {
            debug!("Skipping as it does not support globals");
            continue;
        };

        let mut command;

        match &tool_type {
            ToolType::Rust => {
                command = Command::new("cargo");
                command.arg("install").arg("--force").arg(&dependency);
            }

            ToolType::Plugin(name) => {
                command = Command::new(get_bin_or_fallback(&mut tool).await?);

                // TODO move into plugins
                match name.as_ref() {
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
                        let mut npm = create_tool(&ToolType::Plugin("npm".into())).await?;

                        command = Command::new(get_bin_or_fallback(&mut npm).await?);
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
                    _ => {
                        if let Some(plugin) = tool.as_any().downcast_ref::<SchemaPlugin>() {
                            let Some(args) = &plugin.schema.install.global_args else {
                                return Err(ProtoError::UnsupportedGlobals(plugin.get_name()))?;
                            };

                            for arg in args {
                                if arg == "{dependency}" {
                                    command.arg(&dependency);
                                } else {
                                    command.arg(arg);
                                }
                            }
                        } else {
                            // TODO wasm
                            continue;
                        }
                    }
                };
            }
        };

        let pb = create_progress_bar(label);

        let output = command
            .env("PROTO_INSTALL_GLOBAL", "true")
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
            "{} has been installed to {}!",
            dependency,
            color::path(global_dir),
        );
    }

    Ok(())
}
