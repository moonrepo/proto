mod app;
mod commands;
mod config;
mod helpers;
pub mod tools;

use app::{App, Commands};
use clap::Parser;
use proto_core::color;
use std::process::exit;

#[tokio::main]
async fn main() {
    let app = App::parse();

    let result = match app.command {
        Commands::Bin { tool, semver, shim } => commands::bin(tool, semver, shim).await,
        Commands::Completions { shell } => commands::completions(shell).await,
        Commands::Install { tool, semver, pin } => commands::install(tool, semver, pin).await,
        Commands::Global { tool, semver } => commands::global(tool, semver).await,
        Commands::List { tool } => commands::list(tool).await,
        Commands::ListRemote { tool } => commands::list_remote(tool).await,
        Commands::Local { tool, semver } => commands::local(tool, semver).await,
        Commands::Run {
            tool,
            semver,
            passthrough,
        } => commands::run(tool, semver, passthrough).await,
        Commands::Setup { shell } => commands::setup(shell).await,
        Commands::Uninstall { tool, semver } => commands::uninstall(tool, semver).await,
        Commands::Use => commands::install_all().await,
    };

    if let Err(error) = result {
        eprintln!("{}", color::failure(error.to_string()));
        exit(1);
    }
}
