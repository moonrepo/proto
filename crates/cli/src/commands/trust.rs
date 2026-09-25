use crate::session::{ProtoSession, SessionResult};
use clap::Args;
use proto_core::reporter::NoticeOutput;
use proto_core::{
    PROTO_CONFIG_NAME, ProtoConfig, ProtoTrustSource, get_sensitive_fields, normalize_path,
};
use starbase_console::ui::*;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct TrustArgs {
    #[arg(
        help = "Config file, or directory of config files, to trust. Defaults to the current directory"
    )]
    path: Option<PathBuf>,
}

#[derive(Args, Clone, Debug)]
pub struct UntrustArgs {
    #[arg(
        help = "Config file, or directory of config files, to untrust. Defaults to the current directory"
    )]
    path: Option<PathBuf>,
}

/// What trust applies to: a single config file, or the config files
/// within a directory and its sub-directories.
enum TrustTarget {
    Dir(PathBuf),
    File(PathBuf),
}

impl TrustTarget {
    fn path(&self) -> &Path {
        match self {
            Self::Dir(path) | Self::File(path) => path,
        }
    }

    /// The config files that the target applies to, for reporting. A directory
    /// without configs falls back to its base config, which may be added later.
    fn config_files(&self) -> miette::Result<Vec<PathBuf>> {
        Ok(match self {
            Self::File(file) => vec![file.clone()],
            Self::Dir(dir) => {
                let files = find_config_files(dir)?;

                if files.is_empty() {
                    vec![dir.join(PROTO_CONFIG_NAME)]
                } else {
                    files
                }
            }
        })
    }
}

/// Resolve the target to (un)trust. A path with a config file name is a file,
/// while everything else is a directory. Returns `None` after printing a
/// notice when the path can't be used.
fn resolve_target(
    session: &ProtoSession,
    path: Option<&Path>,
    must_exist: bool,
) -> miette::Result<Option<TrustTarget>> {
    let path = match path {
        Some(path) => session.env.working_dir.join(path),
        None => session.env.working_dir.clone(),
    };

    // Resolve `..` and symlinks, so that messages show the trusted path
    let path = normalize_path(&path);

    if ProtoConfig::is_config_file(&path) {
        if must_exist && !path.is_file() {
            session.console.notice(
                Variant::Caution,
                format!("Config <path>{}</path> does not exist", path.display()),
            )?;

            return Ok(None);
        }

        return Ok(Some(TrustTarget::File(path)));
    }

    if path.is_file() {
        session.console.notice(
            Variant::Caution,
            format!(
                "<path>{}</path> is not a <file>{PROTO_CONFIG_NAME}</file> config file",
                path.display()
            ),
        )?;

        return Ok(None);
    }

    if must_exist && !path.is_dir() {
        session.console.notice(
            Variant::Caution,
            format!("Directory <path>{}</path> does not exist", path.display()),
        )?;

        return Ok(None);
    }

    Ok(Some(TrustTarget::Dir(path)))
}

/// Find the config files (base and environment scoped) within a directory.
fn find_config_files(dir: &Path) -> miette::Result<Vec<PathBuf>> {
    let mut files = vec![];

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let file = entry.path();

            if file.is_file() && ProtoConfig::is_config_file(&file) {
                files.push(file);
            }
        }
    }

    files.sort();

    Ok(files)
}

fn format_fields(fields: &[String]) -> String {
    fields
        .iter()
        .map(|field| format!("<property>{field}</property>"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[instrument(skip(session))]
pub async fn trust(session: ProtoSession, args: TrustArgs) -> SessionResult {
    let Some(target) = resolve_target(&session, args.path.as_deref(), true)? else {
        return Ok(Some(1));
    };

    let probe = match &target {
        TrustTarget::File(file) => file.clone(),
        TrustTarget::Dir(dir) => dir.join(PROTO_CONFIG_NAME),
    };

    if session.env.is_config_owned_by_user(&probe) {
        session.console.notice(
            Variant::Info,
            format!(
                "<path>{}</path> is owned by the user, and is always trusted",
                target.path().display()
            ),
        )?;

        return Ok(None);
    }

    let path = session.env.trust.trust(target.path())?;

    match target {
        TrustTarget::File(file) => {
            let fields = get_sensitive_fields(&ProtoConfig::parse(&file, true)?)?;

            session.console.notice(
                Variant::Success,
                if fields.is_empty() {
                    format!(
                        "Trusted config <path>{}</path>. It has no security-sensitive settings, but any added later will be applied.",
                        path.display()
                    )
                } else {
                    format!(
                        "Trusted config <path>{}</path>. Its security-sensitive settings ({}) will now be applied.",
                        path.display(),
                        format_fields(&fields)
                    )
                },
            )?;
        }
        TrustTarget::Dir(dir) => {
            // List the configs in the directory, so the user knows what was applied
            let mut items = vec![];

            for file in find_config_files(&dir)? {
                let fields = get_sensitive_fields(&ProtoConfig::parse(&file, true)?)?;
                let name = file.file_name().unwrap_or_default().to_string_lossy();

                items.push(if fields.is_empty() {
                    format!(
                        "<file>{name}</file> <mutedlight>(no security-sensitive settings)</mutedlight>"
                    )
                } else {
                    format!(
                        "<file>{name}</file> <mutedlight>({})</mutedlight>",
                        format_fields(&fields)
                    )
                });
            }

            session.console.notice_with(NoticeOutput {
                variant: Variant::Success,
                title: None,
                messages: vec![format!(
                    "Trusted directory <path>{}</path>. The security-sensitive settings of configs within it, and its sub-directories, will now be applied.",
                    path.display()
                )],
                items,
            })?;
        }
    }

    Ok(None)
}

#[instrument(skip(session))]
pub async fn untrust(session: ProtoSession, args: UntrustArgs) -> SessionResult {
    // Allow untrusting paths that no longer exist, so records can be cleaned up
    let Some(target) = resolve_target(&session, args.path.as_deref(), false)? else {
        return Ok(Some(1));
    };

    let path = target.path();

    if session.env.trust.untrust(path)? {
        session.console.notice(
            Variant::Success,
            match &target {
                TrustTarget::Dir(_) => format!(
                    "Untrusted directory <path>{}</path>. The security-sensitive settings of configs within it will no longer be applied.",
                    path.display()
                ),
                TrustTarget::File(_) => format!(
                    "Untrusted config <path>{}</path>. Its security-sensitive settings will no longer be applied.",
                    path.display()
                ),
            },
        )?;
    } else {
        session.console.notice(
            Variant::Info,
            format!(
                "{} <path>{}</path> was not trusted",
                match target {
                    TrustTarget::Dir(_) => "Directory",
                    TrustTarget::File(_) => "Config",
                },
                path.display()
            ),
        )?;
    }

    // The configs may still be trusted through another source
    let subject = match target {
        TrustTarget::Dir(_) => "Configs within it are",
        TrustTarget::File(_) => "The config is",
    };
    let mut reported = vec![];

    for file in target.config_files()? {
        let Some(source) = session.env.trust.get_trust_source(&file) else {
            continue;
        };

        let message = match source {
            ProtoTrustSource::Ci => {
                format!("{subject} still trusted, as all configs are trusted in CI")
            }
            ProtoTrustSource::TrustedPath(path) => format!(
                "{subject} still trusted, as within <path>{}</path> from <property>PROTO_TRUSTED_PATHS</property>",
                path.display()
            ),
            ProtoTrustSource::Record(path) => format!(
                "{subject} still trusted, as <path>{}</path> is trusted. Untrust it with <shell>proto untrust {}</shell>",
                path.display(),
                path.display()
            ),
        };

        if !reported.contains(&message) {
            session.console.notice(Variant::Caution, &message)?;
            reported.push(message);
        }
    }

    Ok(None)
}
