use crate::components::CodeBlock;
use crate::session::{ProtoSession, SessionResult};
use clap::Args;
use iocraft::prelude::*;
use proto_core::{PartialProtoConfig, ProtoConfig, ProtoLock};
use serde::Serialize;
use starbase_console::ui::*;
use starbase_utils::toml;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct DebugConfigArgs {
    #[arg(long, help = "Dump raw configuration objects")]
    raw: bool,
}

#[derive(Serialize)]
struct DebugConfigOutput<'a> {
    config: &'a ProtoConfig,
    files: BTreeMap<&'a PathBuf, &'a PartialProtoConfig>,
    locks: BTreeMap<PathBuf, ProtoLock>,
}

#[instrument(skip(session))]
pub async fn config(session: ProtoSession, args: DebugConfigArgs) -> SessionResult {
    let env = &session.env;
    let manager = env.load_file_manager()?;
    let config = env.load_config()?;

    if args.raw {
        dbg!(manager);
        dbg!(config);

        return Ok(None);
    }

    if session.is_json_format() {
        let mut locks = BTreeMap::default();

        for file in manager.get_config_files() {
            if file.locked
                && let Some(lock) = manager.get_lock(&file.path)?
            {
                locks.insert(lock.path.clone(), (*lock).clone());
            }
        }

        session.console.write_json_for_format(DebugConfigOutput {
            config,
            files: manager
                .get_config_files()
                .into_iter()
                .rev()
                .map(|file| (&file.path, &file.config))
                .collect::<BTreeMap<_, _>>(),
            locks,
        })?;

        return Ok(None);
    }

    // Render from lowest to highest precedence, with each
    // config followed by the lockfile it enabled
    for file in manager.get_config_files().into_iter().rev() {
        if file.exists {
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

        if file.locked
            && let Some(lock) = manager.get_lock(&file.path)?
        {
            let code = toml::format(&*lock, true)?;

            session.console.render(element! {
                Container {
                    Section(
                        title: lock.path.to_string_lossy(),
                        title_color: style_to_color(Style::Path)
                    )
                    CodeBlock(code, format: "toml")
                }
            })?;
        }
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
