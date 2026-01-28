use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use crate::telemetry::{Metric, track_usage};
use crate::utils::tool_record::ToolRecord;
use clap::Args;
use iocraft::element;
use proto_core::flow::lock::Locker;
use proto_core::flow::manage::Manager;
use proto_core::flow::resolve::Resolver;
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

    #[arg(long, help = "Hide uninstall progress output excluding errors")]
    quiet: bool,
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
            })?;
        }
    }

    Ok(())
}

async fn track_uninstall(tool: &Tool, spec: Option<&ToolSpec>) -> Result<(), ProtoCliError> {
    track_usage(
        &tool.proto,
        Metric::UninstallTool {
            id: tool.context.to_string(),
            plugin: tool
                .locator
                .as_ref()
                .map(|loc| loc.to_string())
                .unwrap_or_default(),
            version: match spec {
                Some(spec) => spec.get_resolved_version().to_string(),
                None => "*".into(),
            },
        },
    )
    .await
}

async fn try_uninstall_all(tool: &ToolRecord) -> miette::Result<()> {
    // Loop through each version and uninstall
    let mut manager = Manager::new(tool);

    for version in tool.installed_versions.clone() {
        manager
            .uninstall(&mut ToolSpec::new_resolved(version))
            .await?;
    }

    manager.sync_manifest().await?;

    // Delete inventory
    fs::remove_dir_all(tool.get_inventory_dir())?;
    fs::remove_dir_all(tool.get_temp_dir())?;

    // Remove from lockfile
    Locker::new(tool).remove_from_lockfile()?;

    Ok(())
}

#[instrument(skip(session))]
async fn uninstall_all(session: ProtoSession, args: UninstallArgs) -> AppResult {
    let tool = session.load_tool(&args.context).await?;
    let version_count = tool.inventory.manifest.installed_versions.len();
    let skip_prompts = session.should_skip_prompts();
    let mut confirmed = false;

    if !tool.get_inventory_dir().exists() {
        if !args.quiet {
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
        }

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
                        tool.get_inventory_dir().display()
                    ),
                    on_confirm: &mut confirmed,
                )
            })
            .await?;
    }

    if !skip_prompts && !confirmed {
        return Ok(None);
    }

    debug!("Uninstalling all {} versions", tool.get_name());

    if args.quiet {
        try_uninstall_all(&tool).await?;
    } else {
        let progress = session.render_progress_loader().await;

        progress.set_message(format!("Uninstalling {}", tool.get_name()));

        let result = try_uninstall_all(&tool).await;

        if result.is_ok() {
            progress.set_message(format!("Uninstalled {}", tool.get_name()));
        }

        progress.stop().await?;

        result?;
    }

    unpin_version(&session, &args)?;
    track_uninstall(&tool, None).await?;

    if !args.quiet {
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
    }

    Ok(None)
}

async fn try_uninstall_one(tool: &ToolRecord, spec: &mut ToolSpec) -> miette::Result<()> {
    let mut manager = Manager::new(tool);
    manager.uninstall(spec).await?;
    manager.sync_manifest().await?;

    Ok(())
}

#[instrument(skip(session))]
async fn uninstall_one(
    session: ProtoSession,
    args: UninstallArgs,
    mut spec: ToolSpec,
) -> AppResult {
    let tool = session.load_tool(&args.context).await?;

    Resolver::new(&tool)
        .resolve_version(&mut spec, false)
        .await?;

    if !tool.is_installed(&spec) {
        if !args.quiet {
            session.console.render(element! {
                Notice(variant: Variant::Caution) {
                    StyledText(
                        content: format!(
                            "{} <version>{}</version> has not been installed locally",
                            tool.get_name(),
                            spec.get_resolved_version(),
                        ),
                    )
                }
            })?;
        }

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
                        spec.get_resolved_version(),
                        tool.get_product_dir(&spec).display()
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

    if args.quiet {
        try_uninstall_one(&tool, &mut spec).await?;
    } else {
        let progress = session.render_progress_loader().await;

        progress.set_message(format!(
            "Uninstalling {} <version>{}</version>",
            tool.get_name(),
            spec.get_resolved_version()
        ));

        let result = try_uninstall_one(&tool, &mut spec).await;

        if result.is_ok() {
            progress.set_message(format!("Uninstalled {}", tool.get_name()));
        }

        progress.stop().await?;

        result?;
    }

    unpin_version(&session, &args)?;
    track_uninstall(&tool, Some(&spec)).await?;

    if !args.quiet {
        session.console.render(element! {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: format!(
                        "{} <version>{}</version> has been uninstalled!",
                        tool.get_name(),
                        spec.get_resolved_version(),
                    ),
                )
            }
        })?;
    }

    Ok(None)
}

#[instrument(skip(session))]
pub async fn uninstall(session: ProtoSession, args: UninstallArgs) -> AppResult {
    match args.spec.clone() {
        Some(spec) => uninstall_one(session, args, spec).await,
        None => uninstall_all(session, args).await,
    }
}
