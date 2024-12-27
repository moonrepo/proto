use crate::components::{InstallAllProgress, InstallProgress, InstallProgressProps};
use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use crate::utils::install_graph::*;
use crate::workflows::{InstallOutcome, InstallWorkflow, InstallWorkflowParams};
use clap::Args;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use proto_core::{ConfigMode, Id, PinLocation, Tool, UnresolvedVersionSpec, PROTO_PLUGIN_KEY};
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_console::utils::formats::format_duration;
use starbase_styles::color;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::{spawn, JoinSet};
use tokio::time::sleep;
use tracing::{debug, instrument, trace};

#[derive(Args, Clone, Debug, Default)]
pub struct InstallArgs {
    #[arg(help = "ID of a single tool to install")]
    pub id: Option<Id>,

    #[arg(
        default_value = "latest",
        help = "When installing one tool, the version or alias to install",
        group = "version-type"
    )]
    pub spec: Option<UnresolvedVersionSpec>,

    #[arg(
        long,
        help = "When installing one tool, use a canary (nightly, etc) version",
        group = "version-type"
    )]
    pub canary: bool,

    #[arg(long, help = "Force reinstallation even if it is already installed")]
    pub force: bool,

    #[arg(long, help = "Pin the resolved version to .prototools")]
    pub pin: Option<Option<PinLocation>>,

    // Passthrough args (after --)
    #[arg(
        last = true,
        help = "When installing one tool, additional arguments to pass to the tool"
    )]
    pub passthrough: Vec<String>,

    // Used internally by other commands to trigger conditional logic
    #[arg(hide = true, long)]
    pub internal: bool,
}

impl InstallArgs {
    fn get_pin_location(&self) -> Option<PinLocation> {
        self.pin.as_ref().map(|pin| pin.unwrap_or_default())
    }

    fn get_unresolved_spec(&self) -> UnresolvedVersionSpec {
        if self.canary {
            UnresolvedVersionSpec::Canary
        } else {
            self.spec.clone().unwrap_or_default()
        }
    }
}

pub fn enforce_requirements(
    tool: &Tool,
    versions: &BTreeMap<Id, UnresolvedVersionSpec>,
) -> miette::Result<()> {
    for require_id in &tool.metadata.requires {
        if !versions.contains_key(require_id.as_str()) {
            return Err(ProtoCliError::ToolRequiresNotMet {
                tool: tool.get_name().to_owned(),
                requires: require_id.to_owned(),
            }
            .into());
        }
    }

    Ok(())
}

#[instrument(skip(session))]
pub async fn install_one(session: ProtoSession, args: InstallArgs, id: Id) -> AppResult {
    debug!(id = id.as_str(), "Loading tool");

    let tool = session.load_tool(&id).await?;

    // Load config including global versions,
    // so that our requirements can be satisfied
    if !args.internal {
        let config = session.load_config_with_mode(ConfigMode::UpwardsGlobal)?;

        enforce_requirements(&tool, &config.versions)?;
    }

    // Create our workflow and setup the progress reporter
    let mut workflow = InstallWorkflow::new(tool);
    let reporter = workflow.progress_reporter.clone();
    let console = session.console.clone();

    let handle = spawn(async move {
        console
            .render_loop(element! {
                InstallProgress(reporter)
            })
            .await
    });

    // Wait a bit for the component to be rendered
    sleep(Duration::from_millis(50)).await;

    let result = workflow
        .install(
            args.get_unresolved_spec(),
            InstallWorkflowParams {
                pin_to: args.get_pin_location(),
                force: args.force,
                multiple: false,
                passthrough_args: args.passthrough,
            },
        )
        .await;

    workflow.progress_reporter.exit();
    handle.await.into_diagnostic()??;

    let outcome = result?;
    let tool = workflow.tool;

    if args.internal {
        return Ok(None);
    }

    match outcome {
        InstallOutcome::Installed => {
            session.console.render(element! {
                Notice(variant: Variant::Success) {
                    StyledText(
                        content: format!(
                            "{} <version>{}</version> has been installed to <path>{}</path>!",
                            tool.get_name(),
                            tool.get_resolved_version(),
                            tool.get_product_dir().display(),
                        ),
                    )
                }
            })?;
        }
        InstallOutcome::AlreadyInstalled => {
            session.console.render(element! {
                Notice(variant: Variant::Info) {
                    StyledText(
                        content: format!(
                            "{} <version>{}</version> has already been installed at <path>{}</path>!",
                            tool.get_name(),
                            tool.get_resolved_version(),
                            tool.get_product_dir().display(),
                        ),
                    )
                }
            })?;
        }
        _ => {}
    };

    Ok(None)
}

#[instrument(skip(session))]
async fn install_all(session: ProtoSession, args: InstallArgs) -> AppResult {
    debug!("Loading all tools");

    let tools = session.load_tools().await?;

    debug!("Detecting tool versions to install");

    let mut versions = session.load_config()?.versions.to_owned();
    versions.remove(PROTO_PLUGIN_KEY);

    for tool in &tools {
        if versions.contains_key(&tool.id) {
            continue;
        }

        if let Some((candidate, _)) = tool.detect_version_from(&session.env.working_dir).await? {
            debug!("Detected version {} for {}", candidate, tool.get_name());

            versions.insert(tool.id.clone(), candidate);
        }
    }

    if versions.is_empty() {
        session.console.render(element! {
            Notice(variant: Variant::Caution) {
                StyledText(
                    content: "No versions have been configured, nothing to install!",
                )
                #(if session.env.config_mode == ConfigMode::UpwardsGlobal {
                    None
                } else {
                    Some(element! {
                        View(margin_top: 1) {
                            StyledText(
                                content: format!(
                                    "Configuration has been loaded in <symbol>{}</symbol> mode. Try changing the mode with <property>--config-mode</property> to include other pinned versions.",
                                    session.env.config_mode
                                )
                            )
                        }
                    })
                })
            }
        })?;

        return Ok(Some(1));
    }

    // Filter down tools to only those that have a version
    let tools = tools
        .into_iter()
        .filter(|tool| versions.contains_key(&tool.id))
        .collect::<Vec<_>>();

    // Determine longest ID for use within progress bars
    let longest_id = versions
        .keys()
        .fold(0, |acc, id| acc.max(id.as_str().len()));

    // Then install each tool in parallel!
    let mut topo_graph = InstallGraph::new(&tools);
    let mut progress_rows = BTreeMap::default();
    let mut set = JoinSet::new();
    let started = Instant::now();
    let force = args.force;
    let pin_to = args.get_pin_location();

    for tool in tools {
        enforce_requirements(&tool, &versions)?;

        let Some(version) = versions.get(&tool.id) else {
            continue;
        };

        let tool_id = tool.id.clone();
        let initial_version = version.clone();
        let topo_graph = topo_graph.clone();
        let mut workflow = InstallWorkflow::new(tool);

        // Clone the progress reporters so that we can render
        // multiple progress bars in parallel
        progress_rows.insert(
            tool_id.clone(),
            InstallProgressProps {
                default_message: Some(format!("Preparing {} install…", workflow.tool.get_name())),
                reporter: Some(OwnedOrShared::Shared(Arc::new(
                    workflow.progress_reporter.clone(),
                ))),
            },
        );

        let handle = set.spawn(async move {
            while let Some(status) = topo_graph.check_install_status(&workflow.tool.id).await {
                match status {
                    InstallStatus::ReqFailed(req_id) => {
                        workflow.progress_reporter.set_message(format!(
                            "Requirement <id>{}</id> failed to install",
                            req_id
                        ));

                        // Abort since requirement failed
                        return InstallOutcome::FailedToInstall;
                    }
                    InstallStatus::WaitingOnReqs(waiting_on) => {
                        workflow.progress_reporter.set_message(format!(
                            "Waiting on requirements: {}",
                            waiting_on
                                .into_iter()
                                .map(|req_id| format!("<id>{req_id}</id>"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    InstallStatus::Waiting => {
                        // Sleep
                    }
                };

                sleep(Duration::from_millis(150)).await;
            }

            match workflow
                .install(
                    initial_version,
                    InstallWorkflowParams {
                        force,
                        pin_to,
                        multiple: true,
                        ..Default::default()
                    },
                )
                .await
            {
                Ok(outcome) => {
                    topo_graph.mark_installed(&workflow.tool.id).await;
                    outcome
                }
                Err(error) => {
                    trace!(
                        "Failed to run {} install workflow: {error}",
                        color::id(&workflow.tool.id)
                    );

                    topo_graph.mark_not_installed(&workflow.tool.id).await;
                    InstallOutcome::FailedToInstall
                }
            }
        });

        trace!(
            task_id = handle.id().to_string(),
            "Spawning {} in background task",
            color::id(tool_id)
        );
    }

    let reporter = ProgressReporter::default();
    let reporter_clone = reporter.clone();
    let console = session.console.clone();

    let handle = spawn(async move {
        console
            .render_loop(element! {
                InstallAllProgress(
                    reporter: reporter_clone,
                    tools: progress_rows,
                    id_width: longest_id,
                )
            })
            .await
    });

    // Wait a bit for the component to be rendered
    sleep(Duration::from_millis(50)).await;

    // Start installing tools after the component has rendered!
    topo_graph.proceed();

    let mut installed_count = 0;
    let mut failed_count = 0;

    while let Some(result) = set.join_next_with_id().await {
        match result {
            Err(error) => {
                trace!(
                    task_id = error.id().to_string(),
                    "Spawned task failed: {}",
                    error
                );

                failed_count += 1;
            }
            Ok((task_id, outcome)) => {
                trace!(task_id = task_id.to_string(), "Spawned task successful");

                if matches!(outcome, InstallOutcome::FailedToInstall) {
                    failed_count += 1;
                } else {
                    installed_count += 1;
                }
            }
        };
    }

    reporter.exit();
    handle.await.into_diagnostic()??;

    session.console.render(element! {
        Notice(
            variant: if failed_count == 0 {
                Variant::Success
            } else {
                Variant::Caution
            },
        ) {
            #((installed_count > 0).then(|| {
                element! {
                    StyledText(
                        content: format!(
                            "Installed {} tools in {}!",
                            installed_count,
                            format_duration(started.elapsed(), false),
                        ),
                    )
                }
            }))
            #((failed_count > 0).then(|| {
                element! {
                    StyledText(
                        content: format!("Failed to install {} tools!", failed_count),
                    )
                }
            }))
        }
    })?;

    Ok(None)
}

#[instrument(skip(session))]
pub async fn install(session: ProtoSession, args: InstallArgs) -> AppResult {
    match args.id.clone() {
        Some(id) => install_one(session, args, id).await,
        None => install_all(session, args).await,
    }
}
