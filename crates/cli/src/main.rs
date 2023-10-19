mod app;
mod commands;
mod helpers;
mod shell;

use app::{App as CLI, Commands};
use clap::Parser;
use starbase::{tracing::TracingOptions, App, MainResult};
use starbase_utils::string_vec;
use std::env;
use tracing::{debug, metadata::LevelFilter};

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
        // This swallows logs from extism when enabled
        intercept_log: env::var("PROTO_WASM_LOG").is_err(),
        log_env: "STARBASE_LOG".into(),
        test_env: "PROTO_TEST".into(),
        ..TracingOptions::default()
    });

    debug!(
        args = ?env::args().collect::<Vec<_>>(),
        "Running proto v{}",
        env!("CARGO_PKG_VERSION")
    );

    let mut app = App::new();

    match cli.command {
        Commands::AddPlugin(args) => app.execute_with_args(commands::add_plugin, args),
        Commands::Alias(args) => app.execute_with_args(commands::alias, args),
        Commands::Bin(args) => app.execute_with_args(commands::bin, args),
        Commands::Clean(args) => app.execute_with_args(commands::clean, args),
        Commands::Completions(args) => app.execute_with_args(commands::completions, args),
        Commands::Install(args) => app.execute_with_args(commands::install, args),
        Commands::InstallGlobal(args) => app.execute_with_args(commands::install_global, args),
        Commands::List(args) => app.execute_with_args(commands::list, args),
        Commands::ListGlobal(args) => app.execute_with_args(commands::list_global, args),
        Commands::ListRemote(args) => app.execute_with_args(commands::list_remote, args),
        Commands::Migrate(args) => app.execute_with_args(commands::migrate, args),
        Commands::Outdated(args) => app.execute_with_args(commands::outdated, args),
        Commands::Pin(args) => app.execute_with_args(commands::pin, args),
        Commands::Plugins(args) => app.execute_with_args(commands::plugins, args),
        Commands::RemovePlugin(args) => app.execute_with_args(commands::remove_plugin, args),
        Commands::Run(args) => app.execute_with_args(commands::run, args),
        Commands::Setup(args) => app.execute_with_args(commands::setup, args),
        Commands::Tools(args) => app.execute_with_args(commands::tools, args),
        Commands::Unalias(args) => app.execute_with_args(commands::unalias, args),
        Commands::Uninstall(args) => app.execute_with_args(commands::uninstall, args),
        Commands::UninstallGlobal(args) => app.execute_with_args(commands::uninstall_global, args),
        Commands::Upgrade => app.execute(commands::upgrade),
        Commands::Use => app.execute(commands::install_all),
    };

    app.run().await?;

    Ok(())
}
