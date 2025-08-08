use crate::commands::install::{InstallArgs, install_one};
use crate::error::ProtoCliError;
use crate::helpers::fetch_latest_version;
use crate::session::ProtoSession;
use crate::telemetry::{Metric, track_usage};
use clap::Args;
use iocraft::prelude::element;
use proto_core::{Id, PROTO_PLUGIN_KEY, SemVer, ToolContext, UnresolvedVersionSpec, is_offline};
use semver::Version;
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_styles::color;
use starbase_utils::{fs, fs::FsError, json};
use std::env;
use std::fmt::Debug;
use std::path::Path;
use tracing::{debug, instrument};

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
        let target_chain = format!(
            "<version>{current}</version> <mutedlight>→</mutedlight> <version>{target}</version>"
        );

        session.console.render(element! {
            Notice(variant: Variant::Info) {
                StyledText(content: if target_version == current_version {
                    format!("You're already on version <version>{current}</version> of proto!")
                } else if has_explicit_target {
                    format!("An explicit version of proto will be used: {target_chain}")
                } else if target_version > current_version {
                    format!("A newer version of proto is available: {target_chain}")
                } else {
                    format!("An older version of proto is available: {target_chain}")
                })
            }
        })?;

        return Ok(None);
    }

    // Already on the version, so exit early
    if not_available {
        session.console.render(element! {
            Notice(variant: Variant::Info) {
                StyledText(
                    content: format!("You're already on version <version>{current}</version> of proto!")
                )
            }
        })?;

        return Ok(None);
    }

    // Confirm upgrade if another process is running
    if let Some(pid) = is_running() {
        session.console.render(element! {
            Notice(variant: Variant::Caution) {
                StyledText(
                    content: format!("Another instance of <shell>proto</shell> is currently running with the process ID {}. You may run into issues if you continue.", pid)
                )
            }
        })?;

        let skip_prompts = session.should_skip_prompts();
        let mut confirmed = false;

        if !skip_prompts {
            session
                .console
                .render_interactive(element! {
                    Confirm(
                        label: if target_version >= current_version {
                            "Continue upgrade?"
                        } else {
                            "Continue downgrade?"
                        },
                        on_confirm: &mut confirmed,
                    )
                })
                .await?;
        }

        if !skip_prompts && !confirmed {
            return Ok(None);
        }
    }

    // Load the tool and install the new version
    install_one(
        session.clone(),
        InstallArgs {
            internal: true,
            spec: Some(UnresolvedVersionSpec::Semantic(SemVer(target_version.clone())).into()),
            ..Default::default()
        },
        ToolContext::new(Id::raw(PROTO_PLUGIN_KEY)),
    )
    .await?;

    // Replace the global binaries
    let tool_dir = session.env.store.inventory_dir.join(PROTO_PLUGIN_KEY);
    let is_current_exe_managed = env::current_exe().is_ok_and(|exe| exe.starts_with(&tool_dir));

    let upgraded = replace_binaries(
        tool_dir.join(target_version.to_string()),
        session.env.store.bin_dir.clone(),
        // Don't relocate within our CI pipeline as it causes issues,
        // but do relocate for other user's CI and local development
        !is_current_exe_managed && env::var("PROTO_TEST").is_err(),
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
                    content: if target_version >= current_version {
                        format!("Upgraded proto to <version>{target}</version>!")
                    } else {
                        format!("Downgraded proto to <version>{target}</version>!")
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
    use sysinfo::{ProcessStatus, ProcessesToUpdate};

    let self_pid = std::process::id();

    debug!(
        self_pid = self_pid,
        "Checking if proto is currently running in a separate process"
    );

    let mut system = sysinfo::System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);

    for process in system.processes_by_name("proto".as_ref()) {
        if process.pid().as_u32() == self_pid {
            continue;
        }

        let name = process.name();
        let status = process.status();

        debug!(
            pid = process.pid().as_u32(),
            name = name.to_str(),
            exe = ?process.exe(),
            status = ?status,
            "Found a potential process"
        );

        if (name == "proto"
            || name == "proto-shim"
            || name == "proto.exe"
            || name == "proto-shim.exe")
            && matches!(status, ProcessStatus::Run)
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

#[instrument]
fn replace_binaries(
    source_dir: impl AsRef<Path> + Debug,
    target_dir: impl AsRef<Path> + Debug,
    relocate_current: bool,
) -> Result<bool, ProtoCliError> {
    let source_dir = source_dir.as_ref();
    let target_dir = target_dir.as_ref();
    let bin_names = if cfg!(windows) {
        vec!["proto.exe", "proto-shim.exe"]
    } else {
        vec!["proto", "proto-shim"]
    };

    let mut output_dirs = vec![target_dir.to_path_buf()];

    if relocate_current && let Ok(current) = env::current_exe() {
        let current_dir = current.parent().unwrap();

        if current_dir != target_dir {
            output_dirs.push(current_dir.to_path_buf());
        }
    }

    let mut replaced = false;

    for bin_name in bin_names {
        let input_path = source_dir.join(bin_name);

        if !input_path.exists() {
            continue;
        }

        for output_dir in &output_dirs {
            let output_path = output_dir.join(bin_name);
            let relocate_path = output_dir.join(format!("{bin_name}.backup"));

            if output_path.exists() {
                self_replace(&output_path, &input_path, &relocate_path)?;
            } else {
                fs::copy_file(&input_path, &output_path)?;
                fs::update_perms(&output_path, None)?;
            }

            replaced = true;
        }
    }

    Ok(replaced)
}

#[cfg(unix)]
fn self_replace(
    current_exe: &Path,
    replace_with: &Path,
    relocate_to: &Path,
) -> Result<(), FsError> {
    use std::os::unix::fs::PermissionsExt;

    // If we're a symlink, we need to find the real location and operate on
    // that instead of the link.
    let exe = current_exe.canonicalize().map_err(|error| FsError::Read {
        path: current_exe.to_path_buf(),
        error: Box::new(error),
    })?;
    let perms = fs::metadata(&exe)?.permissions();

    // Relocate the current executable. We do a rename/move as it keeps the
    // same inode's, just changes the literal path. This allows the binary
    // to keep executing without failure. A copy will *not* work!
    fs::rename(exe, relocate_to)?;

    // We then copy the replacement executable to the original location,
    // and attempt to persist the original permissions.
    fs::copy_file(replace_with, current_exe)?;
    fs::update_perms(current_exe, Some(perms.mode()))?;

    Ok(())
}

#[cfg(windows)]
fn self_replace(
    current_exe: &Path,
    replace_with: &Path,
    relocate_to: &Path,
) -> Result<(), FsError> {
    // If we're a symlink, we need to find the real location and operate on
    // that instead of the link.
    let exe = current_exe.canonicalize().map_err(|error| FsError::Read {
        path: current_exe.to_path_buf(),
        error: Box::new(error),
    })?;

    // Relocate the current executable. We do a rename/move as it keeps the
    // same ID/handle, just changes the literal path. This allows the binary
    // to keep executing without failure. A copy will *not* work!
    fs::rename(exe, relocate_to)?;

    // We then copy the replacement executable to a temporary location.
    let mut temp_exe = current_exe.to_path_buf();
    temp_exe.set_extension("temp.exe");

    fs::copy_file(replace_with, &temp_exe)?;

    // And lastly, we move the temporary to the original location. This avoids
    // writing/copying data to the original, and instead does a rename/move.
    fs::rename(temp_exe, current_exe)?;

    Ok(())
}
