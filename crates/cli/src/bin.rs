mod app;
mod commands;
mod helpers;
mod hooks;
mod shell;
pub mod tools;

use app::{App as CLI, Commands};
use clap::Parser;
use starbase::{system, tracing::TracingOptions, App, MainResult, State};
use starbase_utils::string_vec;
use std::env;
use tracing::metadata::LevelFilter;

#[derive(State)]
pub struct CliCommand(pub Commands);

#[system]
async fn run(command: StateRef<CliCommand>) {
    match command.0.clone() {
        Commands::Alias {
            tool,
            alias,
            semver,
        } => commands::alias(tool, alias, semver).await?,
        Commands::Bin { tool, semver, shim } => commands::bin(tool, semver, shim).await?,
        Commands::Clean { days, yes } => commands::clean(days, yes).await?,
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
        Commands::ListGlobal { tool } => commands::list_global(tool).await?,
        Commands::ListRemote { tool } => commands::list_remote(tool).await?,
        Commands::Local { tool, semver } => commands::local(tool, semver).await?,
        Commands::Run {
            tool,
            semver,
            bin,
            passthrough,
        } => commands::run(tool, semver, bin, passthrough).await?,
        Commands::Setup { shell, profile } => commands::setup(shell, profile).await?,
        Commands::Unalias { tool, alias } => commands::unalias(tool, alias).await?,
        Commands::Uninstall { tool, semver } => commands::uninstall(tool, semver).await?,
        Commands::Upgrade => commands::upgrade().await?,
        Commands::Use => commands::install_all().await?,
    };
}

#[tokio::main]
async fn main() -> MainResult {
    App::setup_diagnostics();

    let cli = CLI::parse();

    if let Some(level) = cli.log {
        env::set_var("STARBASE_LOG", level.to_string());
    } else if let Ok(level) = env::var("PROTO_LOG") {
        env::set_var("STARBASE_LOG", level);
    }

    App::setup_tracing_with_options(TracingOptions {
        default_level: if matches!(cli.command, Commands::Bin { .. } | Commands::Run { .. }) {
            LevelFilter::WARN
        } else if matches!(cli.command, Commands::Completions { .. }) {
            LevelFilter::OFF
        } else {
            LevelFilter::INFO
        },
        filter_modules: string_vec!["proto", "starbase", "warpgate"],
        log_env: "STARBASE_LOG".into(),
        test_env: "PROTO_TEST".into(),
        ..TracingOptions::default()
    });

    let mut app = App::new();
    app.set_state(CliCommand(cli.command));
    app.execute(run);
    app.run().await?;

    Ok(())
}
