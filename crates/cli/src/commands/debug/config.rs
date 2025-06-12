use crate::components::CodeBlock;
use crate::session::ProtoSession;
use iocraft::prelude::*;
use proto_core::{ProtoConfig, ProtoConfigFile};
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::{json, toml};

#[derive(Serialize)]
struct DebugConfigResult<'a> {
    config: &'a ProtoConfig,
    files: Vec<&'a ProtoConfigFile>,
}

#[tracing::instrument(skip_all)]
pub async fn config(session: ProtoSession) -> AppResult {
    let env = &session.env;
    let manager = env.load_file_manager()?;
    let config = env.load_config()?;

    if session.should_print_json() {
        let result = DebugConfigResult {
            config,
            files: manager
                .get_config_files()
                .into_iter()
                .rev()
                .collect::<Vec<_>>(),
        };

        session
            .console
            .out
            .write_line(json::format(&result, true)?)?;

        return Ok(None);
    }

    for file in manager.get_config_files().into_iter().rev() {
        if !file.exists {
            continue;
        }

        let code = toml::format(&file.config, true)?;

        session.console.render(element! {
            Container {
                Section(
                    title: file.path.to_string_lossy(),
                    title_color: style_to_color(Style::Path)
                )
                CodeBlock(code, format: "toml")
            }
        })?;
    }

    let code = toml::format(config, true)?;

    session.console.render(element! {
        Container {
            Section(
                title: "Final configuration",
                title_color: style_to_color(Style::Shell), // pink brand
            )
            CodeBlock(code, format: "toml")
        }
    })?;

    Ok(None)
}
