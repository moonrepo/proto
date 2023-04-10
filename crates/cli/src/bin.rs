mod app;
mod commands;
mod helpers;
mod hooks;
mod shell;
pub mod tools;

use app::{App as CLI, Commands};
use clap::Parser;
use helpers::{enable_logging, enable_logging_with_level};
use starbase::{system, App, MainResult, State};

#[derive(State)]
pub struct CliCommand(pub Commands);

#[system]
async fn setup_logging(command: StateRef<CliCommand>) {
    if matches!(command.0, Commands::Bin { .. } | Commands::Run { .. }) {
        enable_logging_with_level("warn");
    } else if matches!(command.0, Commands::Completions { .. }) {
        // Nothing
    } else {
        enable_logging();
    }
}

#[system]
async fn run_command(command: StateMut<CliCommand>) {
    match command.0.clone() {
        Commands::Alias {
            tool,
            alias,
            semver,
        } => commands::alias(tool, alias, semver).await?,
        Commands::Bin { tool, semver, shim } => commands::bin(tool, semver, shim).await?,
        Commands::Completions { shell } => commands::completions(shell).await?,
        Commands::Install {
            tool,
            semver,
            pin,
            passthrough,
        } => commands::install(tool, semver, pin, passthrough).await?,
        Commands::InstallGlobal { tool, dependencies } => {
            commands::install_global(tool, dependencies).await?
        }
        Commands::Global { tool, semver } => commands::global(tool, semver).await?,
        Commands::List { tool } => commands::list(tool).await?,
        Commands::ListRemote { tool } => commands::list_remote(tool).await?,
        Commands::Local { tool, semver } => commands::local(tool, semver).await?,
        Commands::Run {
            tool,
            semver,
            passthrough,
        } => commands::run(tool, semver, passthrough).await?,
        Commands::Setup { shell, profile } => commands::setup(shell, profile).await?,
        Commands::Unalias { tool, alias } => commands::unalias(tool, alias).await?,
        Commands::Uninstall { tool, semver } => commands::uninstall(tool, semver).await?,
        Commands::Upgrade => commands::upgrade().await?,
        Commands::Use => commands::install_all().await?,
    };
}

#[tokio::main]
async fn main() -> MainResult {
    App::setup_hooks("PROTO_LOG");

    let mut app = App::new();
    app.set_state(CliCommand(CLI::parse().command));
    app.startup(setup_logging);
    app.execute(run_command);
    app.run().await?;

    Ok(())
}
