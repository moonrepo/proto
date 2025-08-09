use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use crate::telemetry::{Metric, track_usage};
use clap::Args;
use iocraft::element;
use proto_core::{ProtoConfig, ProtoConfigError, Tool, ToolContext, ToolSpec};
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::fs;
use tracing::{debug, instrument};

#[derive(Args, Clone, Debug)]
pub struct UninstallArgs {
    #[arg(required = true, help = "Tool to uninstall")]
    context: ToolContext,

    #[arg(help = "Version specification to uninstall")]
    spec: Option<ToolSpec>,
}

fn unpin_version(session: &ProtoSession, args: &UninstallArgs) -> Result<(), ProtoConfigError> {
    let manager = session.env.load_file_manager()?;

    for entry in &manager.entries {
        for file in &entry.configs {
            if !file.exists {
                continue;
            }

            ProtoConfig::update_document(&file.path, |doc| {
                if let Some(version) = doc
                    .get(args.context.as_str())
                    .and_then(|item| item.as_str())
                    && (args.spec.is_none()
                        || args
                            .spec
                            .as_ref()
                            .is_some_and(|spec| spec.to_string() == version))
                {
                    doc.as_table_mut().remove(args.context.as_str());
                }

                // if let Some(versions) = &mut config.versions {
                //     let remove = if let Some(version) = versions.get(&args.id) {
                //         args.spec.is_none() || args.spec.as_ref().is_some_and(|spec| spec == version)
                //     } else {
                //         false
                //     };

                //     if remove {
                //         versions.remove(&args.id);
                //     }
                // }
            })?;
        }
    }

    Ok(())
}

async fn track_uninstall(tool: &Tool, all: bool) -> Result<(), ProtoCliError> {
    track_usage(
        &tool.proto,
        Metric::UninstallTool {
            id: tool.context.to_string(),
            plugin: tool
                .locator
                .as_ref()
                .map(|loc| loc.to_string())
                .unwrap_or_default(),
            version: if all {
                "*".into()
            } else {
                tool.get_resolved_version().to_string()
            },
        },
    )
    .await
}

#[instrument(skip(session))]
async fn uninstall_all(session: ProtoSession, args: UninstallArgs) -> AppResult {
    let mut tool = session.load_tool(&args.context).await?;
    let inventory_dir = tool.get_inventory_dir();
    let version_count = tool.inventory.manifest.installed_versions.len();
    let skip_prompts = session.should_skip_prompts();
    let mut confirmed = false;

    if !inventory_dir.exists() {
        session.console.render(element! {
            Notice(variant: Variant::Caution) {
                StyledText(
                    content: format!(
                        "{} has not been installed locally",
                        tool.get_name(),
                    ),
                )
            }
        })?;

        return Ok(Some(1));
    }

    if !skip_prompts {
        session
            .console
            .render_interactive(element! {
                Confirm(
                    label: format!(
                        "Uninstall all {} versions of {} at <path>{}</path>?",
                        version_count,
                        tool.get_name(),
                        inventory_dir.display()
                    ),
                    on_confirm: &mut confirmed,
                )
            })
            .await?;
    }

    if !skip_prompts && !confirmed {
        return Ok(None);
    }

    let progress = session.render_progress_loader().await;

    progress.set_message(format!("Uninstalling {}", tool.get_name()));

    // Delete bins
    for bin in tool.resolve_bin_locations(true).await? {
        session.env.store.unlink_bin(&bin.path)?;
    }

    // Delete shims
    for shim in tool.resolve_shim_locations().await? {
        session.env.store.remove_shim(&shim.path)?;
    }

    // Delete inventory
    fs::remove_dir_all(inventory_dir)?;
    fs::remove_dir_all(tool.get_temp_dir())?;

    // Remove from lockfile
    tool.remove_from_lockfile()?;

    progress.stop().await?;

    unpin_version(&session, &args)?;
    track_uninstall(&tool, true).await?;

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: format!(
                    "{} has been completely uninstalled!",
                    tool.get_name(),
                ),
            )
        }
    })?;

    Ok(None)
}

#[instrument(skip(session))]
async fn uninstall_one(session: ProtoSession, args: UninstallArgs, spec: ToolSpec) -> AppResult {
    let mut tool = session.load_tool(&args.context).await?;

    if !tool.is_setup(&spec).await? {
        session.console.render(element! {
            Notice(variant: Variant::Caution) {
                StyledText(
                    content: format!(
                        "{} <version>{}</version> has not been installed locally",
                        tool.get_name(),
                        tool.get_resolved_version(),
                    ),
                )
            }
        })?;

        return Ok(Some(1));
    }

    let skip_prompts = session.should_skip_prompts();
    let mut confirmed = false;

    if !skip_prompts {
        session
            .console
            .render_interactive(element! {
                Confirm(
                    label: format!(
                        "Uninstall {} version <version>{}</version> at <path>{}</path>?",
                        tool.get_name(),
                        tool.get_resolved_version(),
                        tool.get_product_dir().display()
                    ),
                    on_confirm: &mut confirmed,
                )
            })
            .await?;
    }

    if !skip_prompts && !confirmed {
        return Ok(None);
    }

    debug!("Uninstalling {} with version {}", tool.get_name(), spec);

    let progress = session.render_progress_loader().await;

    progress.set_message(format!(
        "Uninstalling {} <version>{}</version>",
        tool.get_name(),
        tool.get_resolved_version()
    ));

    let result = tool.teardown(&spec).await;

    progress.stop().await?;
    result?;

    unpin_version(&session, &args)?;
    track_uninstall(&tool, false).await?;

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: format!(
                    "{} <version>{}</version> has been uninstalled!",
                    tool.get_name(),
                    tool.get_resolved_version(),
                ),
            )
        }
    })?;

    Ok(None)
}

#[instrument(skip(session))]
pub async fn uninstall(session: ProtoSession, args: UninstallArgs) -> AppResult {
    match args.spec.clone() {
        Some(spec) => uninstall_one(session, args, spec).await,
        None => uninstall_all(session, args).await,
    }
}
