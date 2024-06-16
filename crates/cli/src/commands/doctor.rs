use crate::printer::Printer;
use crate::session::ProtoSession;
use clap::Args;
use serde::Serialize;
use starbase::AppResult;
use starbase_shell::ShellType;
use starbase_styles::color;
use starbase_utils::json;
use std::path::PathBuf;
use std::{env, process};

#[derive(Args, Clone, Debug)]
pub struct DoctorArgs {
    #[arg(long, help = "Shell to diagnose for")]
    shell: Option<ShellType>,

    #[arg(long, help = "Print the diagnosis in JSON format")]
    json: bool,
}

#[derive(Serialize)]
struct Issue {
    issue: String,
    resolution: Option<String>,
    comment: Option<String>,
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
pub async fn doctor(session: ProtoSession, args: DoctorArgs) -> AppResult {
    let shell = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };
    let shell_data = shell.build();
    let shell_path = session
        .env
        .store
        .load_preferred_profile()?
        .unwrap_or_else(|| shell_data.get_env_path(&session.env.home));

    let paths_env = env::var_os("PATH").unwrap_or_default();
    let paths = env::split_paths(&paths_env).collect::<Vec<_>>();

    // Disable ANSI colors in JSON output
    if args.json {
        env::set_var("NO_COLOR", "1");
    }

    let errors = gather_errors(&session, &paths);
    let warnings = gather_warnings(&session, &paths);
    let tips = gather_tips();

    if args.json {
        println!(
            "{}",
            json::format(
                &Diagnosis {
                    shell: shell.to_string(),
                    shell_profile: shell_path,
                    errors,
                    warnings,
                    tips,
                },
                true
            )?
        );

        return Ok(());
    }

    if errors.is_empty() && warnings.is_empty() {
        println!(
            "{}",
            color::success("No issues detected with your proto installation!")
        );

        return Ok(());
    }

    let mut printer = Printer::new();

    printer.line();
    printer.entry("Shell", color::id(shell.to_string()));
    printer.entry(
        "Shell profile",
        color::path(
            session
                .env
                .store
                .load_preferred_profile()?
                .unwrap_or_else(|| shell_data.get_env_path(&session.env.home)),
        ),
    );

    if !errors.is_empty() {
        printer.named_section(color::failure("Errors"), |p| {
            print_issues(&errors, p);

            Ok(())
        })?;
    }

    if !warnings.is_empty() {
        printer.named_section(color::caution("Warnings"), |p| {
            print_issues(&warnings, p);

            Ok(())
        })?;
    }

    if !tips.is_empty() {
        printer.named_section(color::label("Tips"), |p| {
            p.list(tips);

            Ok(())
        })?;
    }

    printer.flush();

    if !errors.is_empty() {
        process::exit(1);
    }

    Ok(())
}

fn gather_errors(session: &ProtoSession, paths: &[PathBuf]) -> Vec<Issue> {
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

    if has_shims_before_bins && found_shims && found_bin {
        errors.push(Issue {
            issue: format!(
                "Bin directory ({}) was found BEFORE the shims directory ({}) on {}",
                color::path(&session.env.store.bin_dir),
                color::path(&session.env.store.shims_dir),
                color::property("PATH")
            ),
            resolution: Some(
                "Ensure the shims path comes before the bin path in your shell".into(),
            ),
            comment: Some(
                "Runtime version detection will not work correctly unless shims are used".into(),
            ),
        })
    }

    errors
}

fn gather_warnings(session: &ProtoSession, paths: &[PathBuf]) -> Vec<Issue> {
    let mut warnings = vec![];

    if !env::var("PROTO_HOME").is_err() {
        warnings.push(Issue {
            issue: format!(
                "Missing {} environment variable",
                color::property("PROTO_HOME")
            ),
            resolution: Some(format!(
                "Export {} from your shell",
                color::label("PROTO_HOME=\"$HOME/.proto\"")
            )),
            comment: Some(format!(
                "Will default to {} if not defined",
                color::file("~/.proto")
            )),
        });
    }

    let has_shims_on_path = paths
        .iter()
        .any(|path| path == &session.env.store.shims_dir);

    if has_shims_on_path {
        warnings.push(Issue {
            issue: format!(
                "Shims directory ({}) not found on {}",
                color::path(&session.env.store.shims_dir),
                color::property("PATH")
            ),
            resolution: Some(format!(
                "Append {} to path in your shell",
                color::label("$PROTO_HOME/shims")
            )),
            comment: Some("If not using shims on purpose, ignore this warning".into()),
        })
    }

    let has_bins_on_path = paths.iter().any(|path| path == &session.env.store.bin_dir);

    if has_bins_on_path {
        warnings.push(Issue {
            issue: format!(
                "Bin directory ({}) not found on {}",
                color::path(&session.env.store.bin_dir),
                color::property("PATH")
            ),
            resolution: Some(format!(
                "Append {} to path in your shell",
                color::label("$PROTO_HOME/bin")
            )),
            comment: None,
        })
    }

    warnings
}

fn gather_tips() -> Vec<String> {
    Vec::from_iter([format!(
        "Run {} to resolve some of these issues!",
        color::shell("proto setup")
    )])
}

fn print_issues(issues: &[Issue], printer: &mut Printer) {
    let length = issues.len() - 1;

    for (index, issue) in issues.iter().enumerate() {
        printer.entry(
            color::muted_light("Issue"),
            format!(
                "{} {}",
                &issue.issue,
                if let Some(comment) = &issue.comment {
                    color::muted_light(format!("({})", comment))
                } else {
                    "".into()
                }
            ),
        );

        if let Some(resolution) = &issue.resolution {
            printer.entry(color::muted_light("Resolution"), resolution);
        }

        if index != length {
            printer.line();
        }
    }
}
