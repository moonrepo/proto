use crate::error::ProtoCliError;
use crate::session::{LoadToolOptions, ProtoSession, SessionResult};
use clap::Args;
use iocraft::prelude::Size;
use miette::IntoDiagnostic;
use proto_core::flow::detect::{Detector, ProtoDetectError};
use proto_core::flow::install::{InstallOptions, Installer, ProtoInstallError};
use proto_core::flow::lock::{LockOutcome, LockSkipReason, Locker};
use proto_core::flow::resolve::Resolver;
use proto_core::reporter::NoticeOutput;
use proto_core::{
    ProtoEnvironment, Tool, ToolContext, ToolSpec, UnresolvedVersionSpec, VersionSpec, load_tool,
};
use proto_pdk_api::Checksum;
use serde::Serialize;
use starbase_console::ui::*;
use starbase_console::utils::formats::format_duration;
use starbase_utils::envx;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Instant;
use system_env::SystemPlatform;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{debug, instrument};

// Pre-builts can be large, so limit how many are downloaded at once
const MAX_CONCURRENT_LOCKS: usize = 8;

#[derive(Args, Clone, Debug, Default)]
pub struct LockArgs {
    #[arg(help = "Single tool to lock")]
    pub context: Option<ToolContext>,

    #[arg(help = "When locking one tool, the version specification to lock")]
    pub spec: Option<ToolSpec>,

    #[arg(
        long,
        help = "Download and verify platforms that have already been locked"
    )]
    pub force: bool,

    #[arg(
        long = "platform",
        value_delimiter = ',',
        help = "Platforms to lock, instead of the current platform and those already locked"
    )]
    pub platforms: Vec<SystemPlatform>,

    #[arg(long, help = "Hide lock output excluding errors")]
    pub quiet: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LockStatus {
    AlreadyLocked,
    Failed,
    Locked,
    Skipped,
}

#[derive(Debug, Serialize)]
pub struct LockItem {
    pub tool: ToolContext,
    pub spec: UnresolvedVersionSpec,
    pub version: Option<VersionSpec>,
    pub platform: Option<SystemPlatform>,
    pub status: LockStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<LockSkipReason>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<Checksum>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl LockItem {
    fn new(context: &ToolContext, spec: &ToolSpec, platform: Option<SystemPlatform>) -> Self {
        Self {
            tool: context.to_owned(),
            spec: spec.req.clone(),
            version: spec.version.clone(),
            platform,
            status: LockStatus::Failed,
            reason: None,
            checksum: None,
            source: None,
            error: None,
        }
    }

    fn failed(mut self, error: String) -> Self {
        self.status = LockStatus::Failed;
        self.error = Some(error);
        self
    }

    fn with_outcome(mut self, outcome: LockOutcome) -> Self {
        match outcome {
            LockOutcome::AlreadyLocked(record) => {
                self.status = LockStatus::AlreadyLocked;
                self.checksum = record.checksum;
            }
            LockOutcome::Locked(record) => {
                self.status = LockStatus::Locked;
                self.checksum = record.checksum;
                self.source = record.source;
            }
            LockOutcome::Skipped(reason) => {
                self.status = LockStatus::Skipped;
                self.reason = Some(reason);
            }
        };

        self
    }
}

fn get_skip_label(reason: LockSkipReason) -> &'static str {
    match reason {
        LockSkipReason::BuildFromSource => "builds from source",
        LockSkipReason::NativeInstall => "installs natively",
        LockSkipReason::NoLockfile => "no lockfile",
        LockSkipReason::NoPrebuilt => "no pre-built",
        LockSkipReason::NoRecord => "not recorded",
    }
}

fn render_table(session: &ProtoSession, items: &[LockItem]) -> miette::Result<()> {
    let get_width = |get_len: fn(&LockItem) -> usize, min: usize| {
        (items.iter().map(get_len).max().unwrap_or_default() + 3).max(min) as u32
    };

    session.console.table(
        vec![
            TableHeader::new(
                "Tool",
                Size::Length(get_width(|item| item.tool.as_str().len(), 10)),
            ),
            TableHeader::new(
                "Version",
                Size::Length(get_width(
                    |item| item.version.as_ref().map_or(3, |v| v.to_string().len()),
                    10,
                )),
            ),
            TableHeader::new(
                "Platform",
                Size::Length(get_width(
                    |item| item.platform.map_or(3, |p| p.to_string().len()),
                    11,
                )),
            ),
            TableHeader::new("Status", Size::Auto),
        ],
        items
            .iter()
            .map(|item| {
                vec![
                    format!("<id>{}</id>", item.tool),
                    match &item.version {
                        Some(version) => version.to_string(),
                        None => "<mutedlight>N/A</mutedlight>".into(),
                    },
                    match &item.platform {
                        Some(platform) => platform.to_string(),
                        None => "<mutedlight>any</mutedlight>".into(),
                    },
                    match item.status {
                        LockStatus::AlreadyLocked => {
                            "<mutedlight>Already locked</mutedlight>".into()
                        }
                        LockStatus::Failed => "<failure>Failed</failure>".into(),
                        LockStatus::Locked => "<success>Locked</success>".into(),
                        LockStatus::Skipped => format!(
                            "<caution>Skipped</caution> <mutedlight>({})</mutedlight>",
                            item.reason.map(get_skip_label).unwrap_or_default()
                        ),
                    },
                ]
            })
            .collect(),
    )?;

    Ok(())
}

/// Load the tool for a single tool, or every tool with a version, and
/// detect the version to lock, like `proto install` does.
async fn load_tools(
    session: &ProtoSession,
    args: &LockArgs,
) -> miette::Result<Vec<(Tool, ToolSpec)>> {
    if let Some(context) = &args.context {
        let tool = session.load_tool(context).await?.tool;

        let spec = match &args.spec {
            Some(spec) => spec.to_owned(),
            None => match Detector::detect(&tool).await {
                Ok((spec, _)) => spec,
                Err(ProtoDetectError::FailedVersionDetect { .. }) => ToolSpec::default(),
                Err(error) => return Err(error.into()),
            },
        };

        return Ok(vec![(tool, spec)]);
    }

    let records = session
        .load_all_tools_with_options(LoadToolOptions {
            detect_version: true,
            ..Default::default()
        })
        .await?;

    Ok(records
        .into_iter()
        .filter_map(|record| {
            let spec = record.detected_version.clone()?;

            Some((record.tool, spec))
        })
        .collect())
}

async fn lock_tool(
    tool: &Tool,
    spec: &ToolSpec,
    force: bool,
) -> Result<LockOutcome, ProtoInstallError> {
    Installer::new(tool, spec)
        .lock(InstallOptions {
            force,
            strategy: tool.metadata.default_install_strategy,
            ..Default::default()
        })
        .await
}

/// Lock the tool for the platform, loading a separate instance of the tool
/// when the platform is not the host's, so that its plugin targets it.
async fn lock_platform(
    env: &ProtoEnvironment,
    tool: &Tool,
    spec: &ToolSpec,
    platform: Option<SystemPlatform>,
    force: bool,
) -> Result<LockOutcome, String> {
    let result = match platform {
        Some(platform) if &platform != env.get_host_platform() => {
            let tool = load_tool(&tool.context, &env.for_platform(platform))
                .await
                .map_err(|error| error.to_string())?;

            lock_tool(&tool, spec, force).await
        }
        _ => lock_tool(tool, spec, force).await,
    };

    result.map_err(|error| error.to_string())
}

#[instrument(skip(session))]
pub async fn lock(session: ProtoSession, args: LockArgs) -> SessionResult {
    if envx::bool_var("PROTO_IMMUTABLE_LOCKFILE") {
        return Err(ProtoCliError::LockImmutableLockfile.into());
    }

    let tools = load_tools(&session, &args).await?;

    if tools.is_empty() {
        return Err(ProtoCliError::NoConfiguredTools.into());
    }

    let progress = if args.quiet || session.is_json_format() {
        None
    } else {
        Some(session.render_progress_loader().await)
    };

    let started = Instant::now();
    let host = *session.env.get_host_platform();
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_LOCKS));
    let mut set = JoinSet::new();
    let mut items = vec![];

    for (mut tool, mut spec) in tools {
        // Avoid resolving a version when it can't be locked anyway
        if !Locker::new(&tool).has_lockfile()? {
            items.push(
                LockItem::new(&tool.context, &spec, None)
                    .with_outcome(LockOutcome::Skipped(LockSkipReason::NoLockfile)),
            );
            continue;
        }

        if let Some(progress) = &progress {
            progress.set_message(format!("Resolving {} version", tool.get_name()));
        }

        // Resolve the version from the lockfile first, then remotely, like an
        // install, and lock every platform to that same version
        spec.resolve_from_lockfile = true;
        spec.resolve_from_manifest = false;
        spec.update_lockfile = false;

        tool.disable_caching();

        if let Err(error) = Resolver::resolve(&tool, &mut spec, false).await {
            items.push(LockItem::new(&tool.context, &spec, None).failed(error.to_string()));
            continue;
        }

        // Tools that ignore the platform have a single record for every platform
        let platforms = if tool.metadata.lock_options.ignore_os_arch {
            vec![None]
        } else if args.platforms.is_empty() {
            let mut platforms = Locker::new(&tool).get_locked_platforms()?;
            platforms.insert(host);
            platforms.into_iter().map(Some).collect()
        } else {
            args.platforms
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(Some)
                .collect()
        };

        debug!(
            tool = tool.context.as_str(),
            version = spec.get_resolved_version().to_string(),
            platforms = ?platforms,
            "Locking tool for platforms",
        );

        let tool = Arc::new(tool);
        let spec = Arc::new(spec);

        for platform in platforms {
            let env = Arc::clone(&session.env);
            let tool = Arc::clone(&tool);
            let spec = Arc::clone(&spec);
            let semaphore = Arc::clone(&semaphore);
            let force = args.force;

            set.spawn(async move {
                let _permit = semaphore.acquire_owned().await;
                let item = LockItem::new(&tool.context, &spec, platform);

                match lock_platform(&env, &tool, &spec, platform, force).await {
                    Ok(outcome) => item.with_outcome(outcome),
                    Err(error) => item.failed(error),
                }
            });
        }
    }

    let total = items.len() + set.len();

    while let Some(result) = set.join_next().await {
        items.push(result.into_diagnostic()?);

        if let Some(progress) = &progress {
            progress.set_message(format!("Locking tools ({}/{total})", items.len()));
        }
    }

    if let Some(progress) = progress {
        progress.stop().await?;
    }

    items.sort_by(|a, d| {
        (a.tool.as_str(), a.platform.map(|p| p.to_string()))
            .cmp(&(d.tool.as_str(), d.platform.map(|p| p.to_string())))
    });

    if items
        .iter()
        .all(|item| item.reason == Some(LockSkipReason::NoLockfile))
    {
        return Err(ProtoCliError::LockMissingLockfile.into());
    }

    // Reconcile lockfiles by removing orphaned records, like installs
    Locker::prune_orphaned_records(&session.env)?;

    let failed = items
        .iter()
        .filter(|item| item.status == LockStatus::Failed)
        .collect::<Vec<_>>();
    let exit_code = (!failed.is_empty()).then_some(1);

    if session.is_json_format() {
        session.console.write_json_for_format(&items)?;

        return Ok(exit_code);
    }

    if !args.quiet {
        render_table(&session, &items)?;
    }

    if !failed.is_empty() {
        session.console.notice_with(NoticeOutput {
            variant: Variant::Caution,
            messages: failed
                .iter()
                .map(|item| {
                    format!(
                        "Failed to lock <id>{}</id> <version>{}</version>{}: {}",
                        item.tool,
                        item.spec,
                        item.platform
                            .map(|platform| format!(" for <symbol>{platform}</symbol>"))
                            .unwrap_or_default(),
                        item.error.as_deref().unwrap_or_default(),
                    )
                })
                .collect(),
            ..Default::default()
        })?;

        return Ok(exit_code);
    }

    if args.quiet {
        return Ok(None);
    }

    let locked = items
        .iter()
        .filter(|item| item.status == LockStatus::Locked)
        .count();

    if locked == 0 {
        session
            .console
            .notice(Variant::Info, "Everything has already been locked!")?;
    } else {
        session.console.notice(
            Variant::Success,
            format!(
                "Locked {locked} {} in {}!",
                if locked == 1 { "record" } else { "records" },
                format_duration(started.elapsed(), false),
            ),
        )?;
    }

    Ok(None)
}
