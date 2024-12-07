use crate::components::{Issue, IssuesList};
use crate::helpers::fetch_latest_version;
use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::{element, Box, Text};
use semver::Version;
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_shell::ShellType;
use starbase_utils::json;
use std::env;
use std::path::PathBuf;

#[derive(Args, Clone, Debug)]
pub struct DiagnoseArgs {
    #[arg(long, help = "Shell to diagnose for")]
    shell: Option<ShellType>,

    #[arg(long, help = "Print the diagnosis in JSON format")]
    json: bool,
}

#[derive(Serialize)]
struct Diagnosis {
    shell: String,
    shell_profile: PathBuf,
    errors: Vec<Issue>,
    warnings: Vec<Issue>,
    tips: Vec<String>,
}

#[tracing::instrument(skip_all)]
pub async fn diagnose(session: ProtoSession, args: DiagnoseArgs) -> AppResult {
    let shell_type = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    let paths = starbase_utils::env::paths();

    // Disable ANSI colors in JSON output
    if args.json {
        env::set_var("NO_COLOR", "1");
    }

    let mut tips = vec![];
    let errors = gather_errors(&session, &paths, &mut tips).await?;
    let warnings = gather_warnings(&session, &paths, &mut tips).await?;

    if args.json {
        let shell = shell_type.build();
        let shell_path = session
            .env
            .store
            .load_preferred_profile()?
            .unwrap_or_else(|| shell.get_env_path(&session.env.home));

        session.console.out.write_line(json::format(
            &Diagnosis {
                shell: shell_type.to_string(),
                shell_profile: shell_path,
                errors,
                warnings,
                tips,
            },
            true,
        )?)?;

        return Ok(None);
    }

    if errors.is_empty() && warnings.is_empty() {
        session.console.render(element! {
            Notice(variant: Variant::Success) {
                Text(content: "No issues detected with your proto installation!")
            }
        })?;

        return Ok(None);
    }

    let has_errors = !errors.is_empty();
    let shell = shell_type.build();
    let shell_path = session
        .env
        .store
        .load_preferred_profile()?
        .unwrap_or_else(|| shell.get_env_path(&session.env.home));

    session.console.render(element! {
        Container {
            Box(margin_bottom: 1) {
                Entry(
                    name: "Shell",
                    value: element! {
                        StyledText(
                            content: shell_type.to_string(),
                            style: Style::Id,
                        )
                    }.into_any()
                )
                Entry(
                    name: "Shell profile",
                    value: element! {
                        StyledText(
                            content: shell_path.to_string_lossy(),
                            style: Style::Path,
                        )
                    }.into_any()
                )
            }
            #(if errors.is_empty() {
                None
            } else {
                Some(element! {
                    Section(title: "Errors", variant: Variant::Failure) {
                        IssuesList(issues: errors)
                    }
                })
            })
            #(if warnings.is_empty() {
                None
            } else {
                Some(element! {
                    Section(title: "Warnings", variant: Variant::Caution) {
                        IssuesList(issues: warnings)
                    }
                })
            })
            #(if tips.is_empty() {
                None
            } else {
                Some(element! {
                    Section(title: "Tips", variant: Variant::Info) {
                        List {
                            #(tips.into_iter().map(|tip| {
                                element! {
                                    ListItem {
                                        StyledText(content: tip)
                                    }
                                }
                            }))
                        }
                    }
                })
            })
        }
    })?;

    Ok(if has_errors { Some(1) } else { None })
}

async fn gather_errors(
    session: &ProtoSession,
    paths: &[PathBuf],
    _tips: &mut [String],
) -> miette::Result<Vec<Issue>> {
    let mut errors = vec![];
    let mut has_shims_before_bins = false;
    let mut found_shims = false;
    let mut found_bin = false;

    for path in paths {
        if path == &session.env.store.shims_dir {
            found_shims = true;

            if !found_bin {
                has_shims_before_bins = true;
            }
        } else if path == &session.env.store.bin_dir {
            found_bin = true;
        }
    }

    if !has_shims_before_bins && found_shims && found_bin {
        errors.push(Issue {
            issue: format!(
                "Bin directory <path>{}</path> was found BEFORE the shims directory <path>{}</path> on <property>PATH</property>",
                session.env.store.bin_dir.display(),
                session.env.store.shims_dir.display(),
            ),
            resolution: Some(
                "Ensure the shims path comes before the bin path in your shell".into(),
            ),
            comment: Some(
                "Runtime version detection will not work correctly unless shims are used".into(),
            ),
        })
    }

    Ok(errors)
}

async fn gather_warnings(
    session: &ProtoSession,
    paths: &[PathBuf],
    tips: &mut Vec<String>,
) -> miette::Result<Vec<Issue>> {
    let mut warnings = vec![];

    let current_version = &session.cli_version;
    let latest_version = fetch_latest_version().await?;

    if Version::parse(current_version).unwrap() < Version::parse(&latest_version).unwrap() {
        warnings.push(Issue {
            issue: format!(
                "Current proto version <hash>{current_version}</hash> is outdated, latest is <hash>{latest_version}</hash>",
            ),
            resolution: Some("Run <shell>proto upgrade</shell> to update".into()),
            comment: None,
        });
    }

    if env::var("PROTO_HOME").is_err() {
        warnings.push(Issue {
            issue: "Missing <property>PROTO_HOME</property> environment variable".into(),
            resolution: Some(
                "Export <shell>PROTO_HOME=\"$HOME/.proto\"</shell> from your shell".into(),
            ),
            comment: Some("Will default to <file>~/.proto</file> if not defined".into()),
        });
    }

    let has_shims_on_path = paths
        .iter()
        .any(|path| path == &session.env.store.shims_dir);

    if !has_shims_on_path {
        warnings.push(Issue {
            issue: format!(
                "Shims directory <path>{}</path> not found on <property>PATH</property>",
                session.env.store.shims_dir.display(),
            ),
            resolution: Some(
                "Append <file>$PROTO_HOME/shims</file> to <property>PATH</property> in your shell"
                    .into(),
            ),
            comment: Some("If not using shims on purpose, ignore this warning".into()),
        })
    }

    let has_bins_on_path = paths.iter().any(|path| path == &session.env.store.bin_dir);

    if !has_bins_on_path {
        warnings.push(Issue {
            issue: format!(
                "Bin directory <path>{}</path> not found on <property>PATH</property>",
                session.env.store.bin_dir.display()
            ),
            resolution: Some(
                "Append <file>$PROTO_HOME/bin</file> to <property>PATH</property> in your shell"
                    .into(),
            ),
            comment: None,
        })
    }

    if !warnings.is_empty() {
        tips.push("Run <shell>proto setup</shell> to resolve some of these issues!".into());
    }

    Ok(warnings)
}
