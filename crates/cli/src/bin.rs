mod app;
mod commands;
mod helpers;
mod hooks;
mod shell;
pub mod tools;

use app::{App as CLI, Commands};
use clap::Parser;
use starbase::{
    system,
    tracing::{metadata::LevelFilter, TracingOptions},
    App, MainResult, State,
};
use starbase_utils::string_vec;

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
        Commands::Clean { days } => commands::clean(days).await?,
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
    let cli = CLI::parse();

    App::setup_diagnostics();

    App::setup_tracing_with_options(TracingOptions {
        default_level: if matches!(cli.command, Commands::Bin { .. } | Commands::Run { .. }) {
            LevelFilter::WARN
        } else if matches!(cli.command, Commands::Completions { .. }) {
            LevelFilter::OFF
        } else {
            LevelFilter::INFO
        },
        env_name: "PROTO_LOG".into(),
        filter_modules: string_vec!["proto", "starbase"],
        ..TracingOptions::default()
    });

    let mut app = App::new();
    app.set_state(CliCommand(cli.command));
    app.execute(run);
    app.run().await?;

    Ok(())
}
