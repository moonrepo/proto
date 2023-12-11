use crate::helpers::ProtoResource;
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{ProtoConfig, ProtoConfigFile};
use serde::Serialize;
use starbase::system;
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
    let contents = toml::to_string_pretty(&value).into_diagnostic()?;

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

#[system]
pub async fn config(args: ArgsRef<DebugConfigArgs>, proto: ResourceRef<ProtoResource>) {
    let manager = proto.env.load_config_manager()?;
    let config = manager.get_merged_config()?;

    if args.json {
        let result = DebugConfigResult {
            config,
            files: manager.files.iter().rev().collect::<Vec<_>>(),
        };

        println!("{}", json::to_string_pretty(&result).into_diagnostic()?);

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
}
