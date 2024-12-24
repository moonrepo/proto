use crate::commands::install::{install_one, InstallArgs};
use crate::error::ProtoCliError;
use crate::helpers::fetch_latest_version;
use crate::session::ProtoSession;
use crate::telemetry::{track_usage, Metric};
use clap::Args;
use iocraft::prelude::element;
use proto_core::{is_offline, Id, PinLocation, SemVer, UnresolvedVersionSpec, PROTO_PLUGIN_KEY};
use proto_installer::*;
use semver::Version;
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
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

    let latest_version = fetch_latest_version().await?;
    let latest = latest_version.to_string();

    let current_version = session.cli_version.clone();
    let current = current_version.to_string();

    let has_explicit_target = args.target.is_some();
    let target_version = args.target.unwrap_or(latest_version);
    let target = target_version.to_string();

    debug!(
        "Comparing target version {} to current version {}",
        color::hash(&target),
        color::hash(&current),
    );

    let not_available = !has_explicit_target && target_version <= current_version
        || target_version == current_version;

    // Output in JSON so other tools can utilize it
    if session.should_print_json() {
        session.console.out.write_line(json::format(
            &UpgradeInfo {
                available: !not_available,
                current_version: current,
                latest_version: latest,
                target_version: target,
            },
            true,
        )?)?;

        return Ok(None);
    }

    // Only compare versions instead of upgrading
    if args.check {
        let target_chain =
            format!("<hash>{current}</hash> <mutedlight>→</mutedlight> <hash>{target}</hash>");

        let content = if target_version == current_version {
            format!("You're already on version <hash>{current}</hash> of proto!")
        } else if has_explicit_target {
            format!("An explicit version of proto will be used: {target_chain}")
        } else if target_version > current_version {
            format!("A newer version of proto is available: {target_chain}")
        } else {
            format!("An older version of proto is available: {target_chain}")
        };

        session.console.render(element! {
            Notice(variant: Variant::Info) {
                StyledText(content)
            }
        })?;

        return Ok(None);
    }

    if not_available {
        session.console.render(element! {
            Notice(variant: Variant::Info) {
                StyledText(
                    content: format!("You're already on version <hash>{current}</hash> of proto!")
                )
            }
        })?;

        return Ok(None);
    }

    // Load the tool and install the new version
    install_one(
        session.clone(),
        InstallArgs {
            spec: Some(UnresolvedVersionSpec::Semantic(SemVer(
                target_version.clone(),
            ))),
            pin: Some(Some(PinLocation::Global)),
            ..Default::default()
        },
        Id::raw(PROTO_PLUGIN_KEY),
    )
    .await?;

    let upgraded = replace_binaries(
        session
            .env
            .store
            .inventory_dir
            .join(PROTO_PLUGIN_KEY)
            .join(target_version.to_string()),
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
        session.console.render(element! {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: if target_version > current_version {
                        format!("Upgraded proto to <hash>{target}</hash>!")
                    } else {
                        format!("Downgraded proto to <hash>{target}</hash>!")
                    }
                )
            }
        })?;

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
