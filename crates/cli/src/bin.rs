mod app;
mod commands;
mod helpers;
mod hooks;
mod shell;
pub mod tools;

use app::{App as CLI, Commands};
use clap::Parser;
use starbase::{diagnose::IntoDiagnostic, App, MainResult};

#[tokio::main]
async fn main() -> MainResult {
    std::env::set_var("RUST_LOG", "trace");
    tracing_subscriber::fmt::init();

    App::setup_hooks();

    let app = CLI::parse();

    let result = match app.command {
        Commands::Alias {
            tool,
            alias,
            semver,
        } => commands::alias(tool, alias, semver).await,
        Commands::Bin { tool, semver, shim } => commands::bin(tool, semver, shim).await,
        Commands::Completions { shell } => commands::completions(shell).await,
        Commands::Install {
            tool,
            semver,
            pin,
            passthrough,
        } => commands::install(tool, semver, pin, passthrough).await,
        Commands::InstallGlobal { tool, dependencies } => {
            commands::install_global(tool, dependencies).await
        }
        Commands::Global { tool, semver } => commands::global(tool, semver).await,
        Commands::List { tool } => commands::list(tool).await,
        Commands::ListRemote { tool } => commands::list_remote(tool).await,
        Commands::Local { tool, semver } => commands::local(tool, semver).await,
        Commands::Run {
            tool,
            semver,
            passthrough,
        } => commands::run(tool, semver, passthrough).await,
        Commands::Setup { shell, profile } => commands::setup(shell, profile).await,
        Commands::Unalias { tool, alias } => commands::unalias(tool, alias).await,
        Commands::Uninstall { tool, semver } => commands::uninstall(tool, semver).await,
        Commands::Upgrade => commands::upgrade().await,
        Commands::Use => commands::install_all().await,
    };

    result.into_diagnostic()
}
