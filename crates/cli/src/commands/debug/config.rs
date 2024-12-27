use crate::session::{ProtoConsole, ProtoSession};
use iocraft::prelude::*;
use proto_core::{ProtoConfig, ProtoConfigFile};
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_styles::color;
use starbase_utils::{json, toml};

#[derive(Serialize)]
struct DebugConfigResult<'a> {
    config: &'a ProtoConfig,
    files: Vec<&'a ProtoConfigFile>,
}

fn print_toml(console: &ProtoConsole, value: impl Serialize) -> miette::Result<()> {
    let contents = toml::format(&value, true)?
        .lines()
        .map(|line| {
            let indented_line = format!("  {line}");

            if line.starts_with('[') {
                indented_line
            } else {
                color::muted_light(indented_line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    console.out.write_newline()?;
    console.out.write_line(contents)?;
    console.out.write_newline()?;

    Ok(())
}

#[tracing::instrument(skip_all)]
pub async fn config(session: ProtoSession) -> AppResult {
    let env = &session.env;
    let manager = env.load_config_manager()?;
    let config = env.load_config()?;

    if session.should_print_json() {
        let result = DebugConfigResult {
            config,
            files: manager.files.iter().rev().collect::<Vec<_>>(),
        };

        session
            .console
            .out
            .write_line(json::format(&result, true)?)?;

        return Ok(None);
    }

    for file in manager.files.iter().rev() {
        if !file.exists {
            continue;
        }

        session.console.render(element! {
            Container {
                Section(
                    title: file.path.to_string_lossy(),
                    title_color: style_to_color(Style::Path)
                )
            }
        })?;

        print_toml(&session.console, &file.config)?;
    }

    session.console.render(element! {
        Container {
            Section(
                title: "Final configuration",
                title_color: style_to_color(Style::Shell), // pink brand
            )
        }
    })?;

    print_toml(&session.console, config)?;

    Ok(None)
}
