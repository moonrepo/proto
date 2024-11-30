use crate::commands::install::{do_install, InstallArgs};
use crate::error::ProtoCliError;
use crate::helpers::{create_progress_bar, fetch_latest_version};
use crate::session::ProtoSession;
use crate::telemetry::{track_usage, Metric};
use clap::Args;
use proto_core::{is_offline, Id, SemVer, UnresolvedVersionSpec};
use proto_installer::*;
use semver::Version;
use serde::Serialize;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::json;
use std::env;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct UpgradeArgs {
    #[arg(help = "Explicit version to upgrade or downgrade to")]
    target: Option<Version>,

    #[arg(long, help = "Check versions only and avoid upgrading")]
    check: bool,

    #[arg(long, help = "Print the upgrade in JSON format")]
    json: bool,
}

#[derive(Serialize)]
struct UpgradeInfo {
    available: bool,
    current_version: String,
    latest_version: String,
    target_version: String,
}

#[tracing::instrument(skip_all)]
pub async fn upgrade(session: ProtoSession, args: UpgradeArgs) -> AppResult {
    if is_offline() {
        return Err(ProtoCliError::UpgradeRequiresInternet.into());
    }

    if let Some(pid) = is_running() {
        return Err(ProtoCliError::CannotUpgradeProtoRunning { pid: pid.as_u32() }.into());
    }

    let latest = fetch_latest_version().await?;

    let current_version = Version::parse(&session.cli_version).unwrap();
    let current = current_version.to_string();

    let has_explicit_target = args.target.is_some();
    let target_version = args
        .target
        .unwrap_or_else(|| Version::parse(&latest).unwrap());
    let target = target_version.to_string();

    debug!(
        "Comparing target version {} to current version {}",
        color::hash(&target),
        color::hash(&current),
    );

    let not_available = !has_explicit_target && target_version <= current_version
        || target_version == current_version;

    // Output in JSON so other tools can utilize it
    if args.json {
        println!(
            "{}",
            json::format(
                &UpgradeInfo {
                    available: !not_available,
                    current_version: current,
                    latest_version: latest,
                    target_version: target,
                },
                true
            )?
        );

        return Ok(None);
    }

    // Only compare versions instead of upgrading
    if args.check {
        let target_chain = format!(
            "{}{}{}",
            color::hash(&current),
            color::muted_light(" -> "),
            color::hash(&target),
        );

        if target_version == current_version {
            println!(
                "You're already on version {} of proto!",
                color::hash(&current)
            );
        } else if has_explicit_target {
            println!(
                "An explicit version of proto will be used: {}",
                target_chain
            );
        } else if target_version > current_version {
            println!("A newer version of proto is available: {}", target_chain);
        } else if target_version < current_version {
            println!("An older version of proto is available: {}", target_chain);
        }

        return Ok(None);
    }

    if not_available {
        println!(
            "You're already on version {} of proto!",
            color::hash(&current)
        );

        return Ok(None);
    }

    // Load the tool and install the new version
    let mut tool = session.load_proto_tool().await?;

    let pb = create_progress_bar(if target_version > current_version {
        format!("Upgrading to {}", target_version)
    } else {
        format!("Downgrading to {}", target_version)
    });

    do_install(
        &mut tool,
        InstallArgs {
            id: Some(Id::raw("proto")),
            spec: Some(UnresolvedVersionSpec::Semantic(SemVer(
                target_version.clone(),
            ))),
            ..Default::default()
        },
        &pb,
    )
    .await?;

    let upgraded = replace_binaries(
        tool.get_product_dir(),
        session.env.store.bin_dir.clone(),
        // Don't relocate within our CI pipeline as it causes issues,
        // but do relocate for other user's CI and local development
        env::var("PROTO_TEST").is_err(),
    )?;

    // Track usage metrics
    track_usage(
        &session.env,
        Metric::UpgradeProto {
            old_version: current.clone(),
            new_version: target.clone(),
        },
    )
    .await?;

    if upgraded {
        if target_version > current_version {
            println!("Upgraded proto to {}!", color::hash(&target));
        } else {
            println!("Downgraded proto to {}!", color::hash(&target));
        }

        return Ok(None);
    }

    Err(ProtoCliError::UpgradeFailed {
        bin: "proto".into(),
    }
    .into())
}

#[cfg(not(debug_assertions))]
fn is_running() -> Option<sysinfo::Pid> {
    debug!("Checking if proto is currently running in a separate process");

    let system = sysinfo::System::new_all();
    let self_pid = std::process::id();

    for process in system.processes_by_name("proto".as_ref()) {
        if process.pid().as_u32() == self_pid {
            continue;
        }

        let name = process.name();

        if name == "proto"
            || name == "proto-shim"
            || name == "proto.exe"
            || name == "proto-shim.exe"
        {
            return Some(process.pid());
        }
    }

    None
}

// Don't check in tests!
#[cfg(debug_assertions)]
fn is_running() -> Option<sysinfo::Pid> {
    None
}
