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
        Commands::Alias { id, alias, semver } => commands::alias(id, alias, semver).await?,
        Commands::Bin { id, semver, shim } => commands::bin(id, semver, shim).await?,
        Commands::Clean { days, yes } => commands::clean(days, yes).await?,
        Commands::Completions { shell } => commands::completions(shell).await?,
        Commands::Install {
            id,
            semver,
            pin,
            passthrough,
        } => commands::install(id, semver, pin, passthrough).await?,
        Commands::InstallGlobal { id, dependencies } => {
            commands::install_global(id, dependencies).await?
        }
        Commands::Global { id, semver } => commands::global(id, semver).await?,
        Commands::List { id } => commands::list(id).await?,
        Commands::ListGlobal { id } => commands::list_global(id).await?,
        Commands::ListRemote { id } => commands::list_remote(id).await?,
        Commands::Local { id, semver } => commands::local(id, semver).await?,
        Commands::Plugins { json } => commands::plugins(json).await?,
        Commands::Run {
            id,
            semver,
            bin,
            passthrough,
        } => commands::run(id, semver, bin, passthrough).await?,
        Commands::Setup { shell, profile } => commands::setup(shell, profile).await?,
        Commands::Unalias { id, alias } => commands::unalias(id, alias).await?,
        Commands::Uninstall { id, semver } => commands::uninstall(id, semver).await?,
        Commands::UninstallGlobal { id, dependencies } => {
            commands::uninstall_global(id, dependencies).await?
        }
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
