use crate::components::{Issue, IssuesList};
use crate::error::ProtoCliError;
use crate::helpers::fetch_latest_version;
use crate::session::{ProtoSession, SessionResult};
use clap::Args;
use iocraft::prelude::{FlexDirection, View, element};
use proto_core::{Id, ToolContext};
use rustc_hash::FxHashMap;
use serde::Serialize;
use starbase_console::ui::*;
use starbase_shell::ShellType;
use starbase_utils::envx;
use std::env;
use std::path::PathBuf;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct DiagnoseArgs {
    #[arg(long, help = "Shell to diagnose for")]
    shell: Option<ShellType>,
}

#[derive(Serialize)]
struct DiagnoseOutput {
    shell: String,
    shell_profile: PathBuf,
    errors: Vec<Issue>,
    warnings: Vec<Issue>,
    tips: Vec<String>,
}

#[instrument(skip(session))]
pub async fn diagnose(session: ProtoSession, args: DiagnoseArgs) -> SessionResult {
    let shell_type = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    let mut tips = vec![];
    let paths = envx::paths();
    let errors = gather_errors(&session, &paths, &mut tips).await?;
    let warnings = gather_warnings(&session, &paths, &mut tips).await?;

    if session.is_json_format() {
        let shell = shell_type.build();
        let shell_path = session
            .env
            .store
            .load_preferred_profile()?
            .unwrap_or_else(|| shell.get_env_path(&session.env.home_dir));

        session.console.write_json_for_format(DiagnoseOutput {
            shell: shell_type.to_string(),
            shell_profile: shell_path,
            errors,
            warnings,
            tips,
        })?;

        return Ok(None);
    }

    if errors.is_empty() && warnings.is_empty() {
        session.console.notice(
            Variant::Success,
            "No issues detected with your proto installation!",
        )?;

        return Ok(None);
    }

    let has_errors = !errors.is_empty();
    let shell = shell_type.build();
    let shell_path = session
        .env
        .store
        .load_preferred_profile()?
        .unwrap_or_else(|| shell.get_env_path(&session.env.home_dir));

    session.console.render(element! {
        Container {
            View(margin_bottom: 1, flex_direction: FlexDirection::Column) {
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
) -> Result<Vec<Issue>, ProtoCliError> {
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
                "Runtime version detection will not work correctly unless shims take priority".into(),
            ),
        });
    }

    Ok(errors)
}

async fn gather_warnings(
    session: &ProtoSession,
    paths: &[PathBuf],
    tips: &mut Vec<String>,
) -> Result<Vec<Issue>, ProtoCliError> {
    let mut warnings = vec![];

    let current_version = &session.cli_version;
    let latest_version = fetch_latest_version().await?;

    if current_version < &latest_version {
        warnings.push(Issue {
            issue: format!(
                "Current proto version <version>{current_version}</version> is outdated, latest is <version>{latest_version}</version>",
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

    // Detect tools that resolve to the same executable name (e.g. the same id
    // provided by different backends). proto links one shim and bin per name,
    // so the others are silently shadowed — a collision precedence can't resolve.
    let config = session
        .env
        .load_config()
        .map_err(|error| ProtoCliError::Config(Box::new(error)))?;

    let mut contexts_by_id: FxHashMap<&Id, Vec<&ToolContext>> = FxHashMap::default();

    for context in config.versions.keys() {
        contexts_by_id.entry(&context.id).or_default().push(context);
    }

    for (id, contexts) in contexts_by_id {
        if contexts.len() < 2 {
            continue;
        }

        let mut names = contexts.iter().map(|ctx| ctx.as_str()).collect::<Vec<_>>();
        names.sort_unstable();

        warnings.push(Issue {
            issue: format!(
                "Multiple configured tools resolve to the executable name <file>{id}</file>: {}",
                names
                    .into_iter()
                    .map(|name| format!("<id>{name}</id>"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            resolution: Some(
                "Keep only one of these tools, as they cannot share the same shim and binary name. Otherwise the tool linked last wins, which is order-dependent."
                    .into(),
            ),
            comment: None,
        });
    }

    warnings.extend(gather_lockfile_warnings(session)?);

    if !warnings.is_empty() {
        tips.push("Run <shell>proto setup</shell> to resolve some of these issues!".into());
    }

    Ok(warnings)
}

fn gather_lockfile_warnings(session: &ProtoSession) -> Result<Vec<Issue>, ProtoCliError> {
    let mut warnings = vec![];
    let manager = session.env.load_file_manager()?;

    for file in manager.get_config_files() {
        if !file.locked {
            continue;
        }

        let Some(lock) = manager.get_lock(&file.path)? else {
            continue;
        };

        // Gather specs defined in the config that owns the lockfile, so
        // that we can detect lockfile records that no longer match it
        let mut config_specs: BTreeMap<&ToolContext, BTreeSet<&UnresolvedVersionSpec>> =
            BTreeMap::default();

        if let Some(versions) = &file.config.versions {
            for (context, spec) in versions {
                config_specs.entry(context).or_default().insert(&spec.req);
            }
        }

        for (id, records) in &lock.tools {
            let mut seen: FxHashMap<String, usize> = FxHashMap::default();

            for record in records {
                let spec_label = record
                    .spec
                    .as_ref()
                    .map(|spec| spec.to_string())
                    .unwrap_or_else(|| "(unknown)".into());

                if record.version.is_none() {
                    warnings.push(Issue {
                        issue: format!(
                            "Record for <id>{id}</id> with spec <version>{spec_label}</version> in lockfile <path>{}</path> is missing a resolved version",
                            lock.path.display(),
                        ),
                        resolution: Some(
                            "Run <shell>proto install</shell> to re-resolve the record".into(),
                        ),
                        comment: None,
                    });
                }

                *seen
                    .entry(format!(
                        "{spec_label}|{}|{}|{}",
                        record
                            .backend
                            .as_ref()
                            .map(|backend| backend.as_str())
                            .unwrap_or_default(),
                        record.os.map(|os| os.to_string()).unwrap_or_default(),
                        record.arch.map(|arch| arch.to_string()).unwrap_or_default(),
                    ))
                    .or_insert(0) += 1;

                // Only tools defined in a sibling config can be verified,
                // as records may also exist for ad-hoc installs
                let config_context = config_specs.as_ref().and_then(|specs| {
                    specs
                        .iter()
                        .find(|(context, _)| context.id == *id && context.backend == record.backend)
                });

                if let Some((context, specs)) = config_context
                    && let Some(spec) = &record.spec
                    && !specs.contains(spec)
                {
                    warnings.push(Issue {
                        issue: format!(
                            "Record for <id>{context}</id> with spec <version>{spec_label}</version> in lockfile <path>{}</path> does not match any configured version",
                            lock.path.display(),
                        ),
                        resolution: Some(
                            "Run <shell>proto install</shell> to prune the record, or configure the version again"
                                .into(),
                        ),
                        comment: Some("The record is stale and will never be used".into()),
                    });
                }
            }

            for (key, count) in seen {
                if count > 1 {
                    let spec_label = key.split('|').next().unwrap_or_default();

                    warnings.push(Issue {
                        issue: format!(
                            "Found {count} duplicate records for <id>{id}</id> with spec <version>{spec_label}</version> in lockfile <path>{}</path>",
                            lock.path.display(),
                        ),
                        resolution: Some("Remove the duplicate records from the lockfile".into()),
                        comment: None,
                    });
                }
            }
        }
    }

    Ok(warnings)
}
