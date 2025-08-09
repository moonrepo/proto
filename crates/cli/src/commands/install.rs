use crate::error::ProtoCliError;
use crate::helpers::join_list;
use crate::session::{LoadToolOptions, ProtoSession};
use crate::utils::install_graph::*;
use crate::utils::tool_record::ToolRecord;
use crate::workflows::{InstallOutcome, InstallWorkflowManager, InstallWorkflowParams};
use clap::Args;
use iocraft::prelude::element;
use proto_core::{ConfigMode, Id, PinLocation, Tool, ToolContext, ToolSpec};
use proto_pdk_api::{InstallStrategy, PluginFunction};
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_console::utils::formats::format_duration;
use starbase_styles::color;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;
use tokio::time::sleep;
use tracing::{debug, info, instrument, trace};

#[derive(Args, Clone, Debug, Default)]
pub struct InstallArgs {
    #[arg(help = "Single tool to install")]
    pub context: Option<ToolContext>,

    #[arg(
        help = "When installing one tool, the version specification to install",
        group = "version-type"
    )]
    pub spec: Option<ToolSpec>,

    #[arg(
        long,
        help = "Build from source instead of downloading a pre-built",
        group = "build-type"
    )]
    pub build: bool,

    #[arg(
        long,
        help = "Download a pre-built instead of building from source",
        group = "build-type"
    )]
    pub no_build: bool,

    #[arg(long, help = "Force reinstallation even if already installed")]
    pub force: bool,

    #[arg(long, help = "Pin the resolved version to .prototools")]
    pub pin: Option<Option<PinLocation>>,

    // Passthrough args (after --)
    #[arg(
        last = true,
        help = "When installing one tool, additional arguments to pass to the tool"
    )]
    pub passthrough: Vec<String>,

    #[arg(long, help = "Hide install progress output excluding errors")]
    pub quiet: bool,

    #[arg(
        long,
        help = "Don't inherit a version from the lockfile and update the record"
    )]
    pub update_lockfile: bool,

    // Used internally by other commands to trigger conditional logic
    #[arg(hide = true, long)]
    pub internal: bool,
}

impl InstallArgs {
    async fn filter_tools(&self, tools: Vec<ToolRecord>) -> Vec<ToolRecord> {
        let mut list = vec![];

        if self.build {
            info!("Build mode enabled. Only tools that support build from source will install.");

            for tool in tools {
                if tool
                    .plugin
                    .has_func(PluginFunction::BuildInstructions)
                    .await
                {
                    list.push(tool);
                }
            }
        } else if self.no_build {
            info!("Prebuilt mode enabled. Only tools that support prebuilts will install.");

            for tool in tools {
                if tool.plugin.has_func(PluginFunction::DownloadPrebuilt).await {
                    list.push(tool);
                }
            }
        }

        list
    }

    fn get_strategy(&self) -> Option<InstallStrategy> {
        if self.build {
            Some(InstallStrategy::BuildFromSource)
        } else if self.no_build {
            Some(InstallStrategy::DownloadPrebuilt)
        } else {
            None
        }
    }

    fn get_pin_location(&self) -> Option<PinLocation> {
        self.pin.as_ref().map(|pin| pin.unwrap_or_default())
    }
}

pub fn enforce_requirements(
    tool: &Tool,
    versions: &BTreeMap<ToolContext, ToolSpec>,
) -> Result<(), ProtoCliError> {
    for require_id in &tool.metadata.requires {
        let require_ctx = ToolContext::parse(require_id)?;

        if !versions.contains_key(&require_ctx) {
            return Err(ProtoCliError::InstallRequirementsNotMet {
                tool: tool.get_name().to_owned(),
                requires: require_id.to_owned(),
            });
        }
    }

    Ok(())
}

#[instrument(skip(session))]
pub async fn install_one(
    session: ProtoSession,
    args: InstallArgs,
    context: ToolContext,
) -> AppResult {
    debug!(tool = context.as_str(), "Loading tool");

    let tool = session.load_tool(&context).await?;

    // Attempt to detect a version if one was not provided,
    // otherwise fallback to "latest"
    let mut spec = if let Some(spec) = &args.spec {
        spec.to_owned()
    } else if let Some((spec, _)) = tool.detect_version_from(&session.env.working_dir).await? {
        spec.into()
    } else if let Some(spec) = session.load_config()?.versions.get(&context) {
        spec.to_owned()
    } else {
        ToolSpec::default()
    };

    // Don't resolve the version from the manifest
    spec.resolve_from_manifest = false;

    // Don't resolve the version from a lockfile
    spec.read_lockfile = !args.update_lockfile;
    spec.write_lockfile = !args.internal;

    // Load config including global versions,
    // so that our requirements can be satisfied
    if !args.internal {
        let config = session.load_config_with_mode(ConfigMode::UpwardsGlobal)?;

        enforce_requirements(&tool, &config.versions)?;
    }

    // Create our workflow and setup the progress reporter
    let mut workflow_manager = InstallWorkflowManager::new(session.console.clone(), args.quiet);
    let mut workflow = workflow_manager.create_workflow(tool);

    if workflow.is_build(args.get_strategy()) {
        if !args.quiet {
            session.console.render(element! {
                Notice(variant: Variant::Caution) {
                    StyledText(
                        content: "Building from source is currently unstable. Please report general issues to <url>https://github.com/moonrepo/proto</url>",
                    )
                    StyledText(
                        content: "and tool specific issues to <url>https://github.com/moonrepo/plugins</url>.",
                    )
                }
            })?;
        }
    } else {
        workflow_manager.render_single_progress().await;
    }

    let result = workflow
        .install_with_logging(
            spec,
            InstallWorkflowParams {
                log_writer: None,
                pin_to: args.get_pin_location(),
                strategy: args.get_strategy(),
                force: args.force,
                multiple: false,
                passthrough_args: args.passthrough,
                quiet: args.quiet,
                skip_prompts: session.should_skip_prompts(),
            },
        )
        .await;

    workflow_manager.stop_rendering().await?;

    let outcome = result?;
    let tool = workflow.tool;

    if args.internal || args.quiet {
        session.console.err.flush()?;
        session.console.out.flush()?;

        return Ok(None);
    }

    match outcome {
        InstallOutcome::Installed(_) => {
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
        InstallOutcome::AlreadyInstalled(_) => {
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
    debug!("Loading all tools and detecting versions to install");

    let mut versions = BTreeMap::default();
    let tools = session
        .load_all_tools_with_options(LoadToolOptions {
            detect_version: true,
            ..Default::default()
        })
        .await?;

    for tool in &tools {
        if let Some(candidate) = &tool.detected_version {
            debug!("Detected version {} for {}", candidate, tool.get_name());

            versions.insert(tool.context.clone(), candidate.to_owned());
        }
    }

    // Filter down tools to only those that have a version
    let mut tools = tools
        .into_iter()
        .filter(|tool| versions.contains_key(&tool.context))
        .collect::<Vec<_>>();

    // And handle build/prebuilt modes
    if args.build || args.no_build {
        tools = args.filter_tools(tools).await;
    }

    if tools.is_empty() {
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

    // Then install each tool in parallel!
    let mut topo_graph = InstallGraph::new(&tools);
    let mut workflow_manager = InstallWorkflowManager::new(session.console.clone(), args.quiet);
    let mut set = JoinSet::new();
    let started = Instant::now();
    let force = args.force;
    let pin_to = args.get_pin_location();
    let skip_prompts = session.should_skip_prompts();
    let strategy = args.get_strategy();

    for tool in tools {
        enforce_requirements(&tool, &versions)?;

        let Some(version) = versions.get(&tool.context) else {
            continue;
        };

        let mut spec = version.clone();
        spec.read_lockfile = !args.update_lockfile;
        spec.write_lockfile = !args.internal;

        let tool_context = tool.context.clone();
        let topo_graph = topo_graph.clone();
        let mut workflow = workflow_manager.create_workflow(tool);

        let handle = set.spawn(async move {
            while let Some(status) = topo_graph
                .check_install_status(workflow.tool.get_id())
                .await
            {
                match status {
                    InstallStatus::ReqFailed(req_id) => {
                        workflow.progress_reporter.set_message(format!(
                            "Requirement <id>{req_id}</id> failed to install"
                        ));

                        // Abort since requirement failed
                        return InstallOutcome::FailedToInstall(workflow.tool.get_id().clone());
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
                .install_with_logging(
                    spec,
                    InstallWorkflowParams {
                        force,
                        log_writer: None,
                        multiple: true,
                        passthrough_args: vec![],
                        pin_to,
                        skip_prompts,
                        strategy,
                        quiet: args.quiet,
                    },
                )
                .await
            {
                Ok(outcome) => {
                    topo_graph.mark_installed(workflow.tool.get_id()).await;
                    outcome
                }
                Err(error) => {
                    trace!(
                        "Failed to run {} install workflow: {error}",
                        color::id(workflow.tool.get_id())
                    );

                    topo_graph.mark_not_installed(workflow.tool.get_id()).await;
                    InstallOutcome::FailedToInstall(workflow.tool.get_id().clone())
                }
            }
        });

        trace!(
            task_id = handle.id().to_string(),
            "Spawning {} in background task",
            color::id(tool_context)
        );
    }

    workflow_manager.render_multiple_progress().await;
    topo_graph.proceed();

    let mut installed = vec![];
    let mut failed = vec![];

    while let Some(result) = set.join_next_with_id().await {
        match result {
            Err(error) => {
                trace!(
                    task_id = error.id().to_string(),
                    "Spawned task failed: {}", error
                );

                // What to do here?
                failed.push(Id::raw("unknown"));
            }
            Ok((task_id, outcome)) => {
                trace!(task_id = task_id.to_string(), "Spawned task successful");

                match outcome {
                    InstallOutcome::FailedToInstall(id) => {
                        failed.push(id);
                    }
                    InstallOutcome::AlreadyInstalled(id) | InstallOutcome::Installed(id) => {
                        installed.push(id);
                    }
                }
            }
        };
    }

    workflow_manager.stop_rendering().await?;

    let installed_count = installed.len();
    let failed_count = failed.len();

    if args.quiet && failed_count == 0 {
        return Ok(None);
    }

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
                            "Installed {} in {}!",
                            join_list(installed.into_iter().map(color::id).collect::<Vec<_>>()),
                            format_duration(started.elapsed(), false),
                        ),
                    )
                }
            }))
            #((failed_count > 0).then(|| {
                element! {
                    StyledText(
                        content: format!(
                            "Failed to install {}! A log has been written to the current directory.",
                            join_list(failed.into_iter().map(color::id).collect::<Vec<_>>()),
                        ),
                    )
                }
            }))
        }
    })?;

    Ok(Some(failed_count as u8))
}

#[instrument(skip(session))]
pub async fn install(session: ProtoSession, args: InstallArgs) -> AppResult {
    match args.context.clone() {
        Some(context) => install_one(session, args, context).await,
        None => install_all(session, args).await,
    }
}
