mod app;
mod commands;
mod helpers;
mod hooks;
mod shell;
mod states;
pub mod tools;

use app::{App as CLI, Commands};
use clap::Parser;
use proto_core::{ToolsConfig as InnerToolsConfig, UserConfig as InnerUserConfig};
use starbase::{system, tracing::TracingOptions, App, MainResult, State};
use starbase_utils::string_vec;
use states::{ToolsConfig, UserConfig};
use std::env;
use tracing::metadata::LevelFilter;

#[derive(State)]
pub struct CliCommand(pub Commands);

#[system]
pub fn load_configs(state: StatesMut) {
    state.set(UserConfig(InnerUserConfig::load()?));

    state.set(ToolsConfig(InnerToolsConfig::load_upwards(
        env::current_dir().expect("Missing current directory."),
    )?));
}

#[system]
async fn run(
    command: StateRef<CliCommand>,
    tools_config: StateRef<ToolsConfig>,
    user_config: StateRef<UserConfig>,
) {
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
            passthrough,
        } => commands::run(tool, semver, passthrough, user_config).await?,
        Commands::Setup { shell, profile } => commands::setup(shell, profile).await?,
        Commands::Unalias { tool, alias } => commands::unalias(tool, alias).await?,
        Commands::Uninstall { tool, semver } => commands::uninstall(tool, semver).await?,
        Commands::Upgrade => commands::upgrade().await?,
        Commands::Use => commands::install_all(tools_config, user_config).await?,
    };
}

#[tokio::main]
async fn main() -> MainResult {
    App::setup_diagnostics();

    let cli = CLI::parse();

    App::setup_tracing_with_options(TracingOptions {
        default_level: if matches!(cli.command, Commands::Bin { .. } | Commands::Run { .. }) {
            LevelFilter::WARN
        } else if matches!(cli.command, Commands::Completions { .. }) {
            LevelFilter::OFF
        } else {
            LevelFilter::INFO
        },
        filter_modules: string_vec!["proto", "starbase"],
        log_env: "PROTO_LOG".into(),
        test_env: "PROTO_TEST".into(),
        ..TracingOptions::default()
    });

    let mut app = App::new();
    app.set_state(CliCommand(cli.command));
    app.startup(load_configs);
    app.execute(run);
    app.run().await?;

    Ok(())
}
