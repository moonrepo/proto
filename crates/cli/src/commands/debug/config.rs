use crate::session::ProtoSession;
use clap::Args;
use proto_core::{ProtoConfig, ProtoConfigFile};
use serde::Serialize;
use starbase::AppResult;
use starbase_styles::color::{self, OwoStyle};
use starbase_utils::{json, toml};

#[derive(Serialize)]
pub struct DebugConfigResult<'a> {
    config: &'a ProtoConfig,
    files: Vec<&'a ProtoConfigFile>,
}

#[derive(Args, Clone, Debug)]
pub struct DebugConfigArgs {
    #[arg(long, help = "Print the list in JSON format")]
    json: bool,
}

fn print_toml(value: impl Serialize) -> miette::Result<()> {
    let contents = toml::format(&value, true)?;

    let contents = contents
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

    println!("{}", contents);

    Ok(())
}

pub async fn config(session: ProtoSession, args: DebugConfigArgs) -> AppResult {
    let manager = session.env.load_config_manager()?;
    let config = manager.get_merged_config()?;

    if args.json {
        let result = DebugConfigResult {
            config,
            files: manager.files.iter().rev().collect::<Vec<_>>(),
        };

        println!("{}", json::format(&result, true)?);

        return Ok(());
    }

    for file in manager.files.iter().rev() {
        if !file.exists {
            continue;
        }

        println!();
        println!("{}", OwoStyle::new().bold().style(color::path(&file.path)));
        print_toml(&file.config)?;
    }

    println!();
    println!(
        "{}",
        OwoStyle::new().bold().style(color::id("Configuration"))
    );
    print_toml(config)?;
    println!();

    Ok(())
}
