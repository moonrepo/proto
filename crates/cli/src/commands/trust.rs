use crate::session::{ProtoSession, SessionResult};
use clap::Args;
use proto_core::reporter::NoticeOutput;
use proto_core::{PROTO_CONFIG_NAME, ProtoConfig, ProtoConfigSensitive};
use starbase_console::ui::*;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct TrustArgs {
    #[arg(
        help = "Config file, or directory containing config files, to trust. Defaults to the current directory"
    )]
    path: Option<PathBuf>,
}

#[derive(Args, Clone, Debug)]
pub struct UntrustArgs {
    #[arg(
        help = "Config file, or directory containing config files, to untrust. Defaults to the current directory"
    )]
    path: Option<PathBuf>,
}

/// Find the config files to (un)trust: the provided config file itself,
/// or all config files (base and environment scoped) in the provided directory.
fn find_config_files(session: &ProtoSession, path: Option<&Path>) -> miette::Result<Vec<PathBuf>> {
    let path = match path {
        Some(path) => session.env.working_dir.join(path),
        None => session.env.working_dir.clone(),
    };

    if ProtoConfig::is_config_file(&path) {
        return Ok(vec![path]);
    }

    let mut files = vec![];

    if path.is_dir() {
        for entry in fs::read_dir(&path)? {
            let file = entry.path();

            if file.is_file() && ProtoConfig::is_config_file(&file) {
                files.push(file);
            }
        }
    }

    files.sort();

    Ok(files)
}

#[instrument(skip(session))]
pub async fn trust(session: ProtoSession, args: TrustArgs) -> SessionResult {
    let files = find_config_files(&session, args.path.as_deref())?;

    if files.is_empty() {
        session.console.notice(
            Variant::Caution,
            format!("No <file>{PROTO_CONFIG_NAME}</file> config files found to trust"),
        )?;

        return Ok(Some(1));
    }

    for file in files {
        if !file.exists() {
            session.console.notice(
                Variant::Caution,
                format!("Config <path>{}</path> does not exist", file.display()),
            )?;

            continue;
        }

        if !session.env.requires_config_trust(&file) {
            session.console.notice(
                Variant::Info,
                format!(
                    "Config <path>{}</path> is owned by the user, and is always trusted",
                    file.display()
                ),
            )?;

            continue;
        }

        let Some(sensitive) = ProtoConfigSensitive::from_config(&ProtoConfig::parse(&file, true)?)?
        else {
            session.console.notice(
                Variant::Info,
                format!(
                    "Config <path>{}</path> has no security-sensitive settings, and does not need to be trusted",
                    file.display()
                ),
            )?;

            continue;
        };

        session.env.trust.trust(&file, &sensitive.hash)?;

        session.console.notice_with(NoticeOutput {
            variant: Variant::Success,
            title: None,
            messages: vec![format!(
                "Trusted config <path>{}</path>, its following settings will now be applied:",
                file.display()
            )],
            items: sensitive
                .fields
                .iter()
                .map(|field| format!("<property>{field}</property>"))
                .collect(),
        })?;
    }

    Ok(None)
}

#[instrument(skip(session))]
pub async fn untrust(session: ProtoSession, args: UntrustArgs) -> SessionResult {
    let files = find_config_files(&session, args.path.as_deref())?;

    if files.is_empty() {
        session.console.notice(
            Variant::Caution,
            format!("No <file>{PROTO_CONFIG_NAME}</file> config files found to untrust"),
        )?;

        return Ok(Some(1));
    }

    for file in files {
        if session.env.trust.untrust(&file)? {
            session.console.notice(
                Variant::Success,
                format!(
                    "Untrusted config <path>{}</path>, its security-sensitive settings will no longer be applied",
                    file.display()
                ),
            )?;
        } else {
            session.console.notice(
                Variant::Info,
                format!("Config <path>{}</path> was not trusted", file.display()),
            )?;
        }
    }

    Ok(None)
}
