use crate::components::CodeBlock;
use crate::session::{ProtoSession, SessionResult};
use clap::Args;
use iocraft::prelude::element;
use proto_core::reporter::NoticeOutput;
use proto_core::{
    PROTO_CONFIG_NAME, ProtoConfig, get_sensitive_config, get_sensitive_fields, normalize_path,
};
use starbase_console::ui::*;
use starbase_utils::{fs, toml};
use std::path::{Path, PathBuf};
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct TrustArgs {
    #[arg(
        help = "Config file, or directory of config files, to trust. Either absolute, or relative to the current directory. Defaults to the current directory"
    )]
    path: Option<PathBuf>,
}

/// What trust applies to: a single config file, or the config files
/// within a directory and its sub-directories.
pub enum TrustTarget {
    Dir(PathBuf),
    File(PathBuf),
}

impl TrustTarget {
    pub fn path(&self) -> &Path {
        match self {
            Self::Dir(path) | Self::File(path) => path,
        }
    }

    /// The config files that the target applies to, for reporting. A directory
    /// without configs falls back to its base config, which may be added later.
    pub fn config_files(&self) -> miette::Result<Vec<PathBuf>> {
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
pub fn resolve_target(
    session: &ProtoSession,
    path: Option<&Path>,
    must_exist: bool,
) -> miette::Result<Option<TrustTarget>> {
    // Joining an absolute path replaces the working directory
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

/// Render the security-sensitive settings of the provided config files, so
/// that the user can review exactly what is now applied.
fn render_sensitive_settings(session: &ProtoSession, files: &[PathBuf]) -> miette::Result<()> {
    // Notices are structured in JSON formats, but rendered elements are not
    if session.is_json_format() {
        return Ok(());
    }

    for file in files {
        let Some(settings) = get_sensitive_config(&ProtoConfig::parse(file, true)?)? else {
            continue;
        };

        let code = toml::format(&settings, true)?;

        session.console.render(element! {
            Container {
                Section(
                    title: file.to_string_lossy(),
                    title_color: style_to_color(Style::Path)
                )
                CodeBlock(code, format: "toml")
            }
        })?;
    }

    Ok(())
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

    if let TrustTarget::File(file) = &target
        && session.env.is_config_owned_by_user(file)
    {
        session.console.notice(
            Variant::Info,
            format!(
                "Config <path>{}</path> is owned by the user, and is always trusted",
                file.display()
            ),
        )?;

        return Ok(None);
    }

    // Trusting a directory is allowed at any level, but make the scope clear
    if let TrustTarget::Dir(dir) = &target
        && session.env.home_dir.starts_with(dir)
    {
        session.console.notice(
            Variant::Caution,
            "This trusts every config within your home directory, including configs in repositories cloned in the future",
        )?;
    }

    let path = session.env.trust.trust(target.path())?;

    render_sensitive_settings(
        &session,
        &match &target {
            TrustTarget::File(file) => vec![file.clone()],
            TrustTarget::Dir(dir) => find_config_files(dir)?,
        },
    )?;

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
