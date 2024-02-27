mod app;
mod commands;
mod error;
mod helpers;
mod printer;
mod shell;
mod systems;
mod telemetry;

use app::{App as CLI, Commands, DebugCommands, ToolCommands};
use clap::Parser;
use starbase::{tracing::TracingOptions, App, MainResult};
use starbase_utils::string_vec;
use std::env;
use tracing::{debug, metadata::LevelFilter};

#[tokio::main]
async fn main() -> MainResult {
    App::setup_diagnostics();

    let cli = CLI::parse();
    let version = env!("CARGO_PKG_VERSION");

    if let Some(level) = cli.log {
        env::set_var("STARBASE_LOG", level.to_string());
    } else if let Ok(level) = env::var("PROTO_LOG") {
        env::set_var("STARBASE_LOG", level);
    }

    env::set_var("PROTO_VERSION", version);

    let mut modules = string_vec!["proto", "schematic", "starbase", "warpgate"];

    if env::var("PROTO_WASM_LOG").is_ok() {
        modules.push("extism".into());
    } else {
        modules.push("extism::pdk".into());
    }

    App::setup_tracing_with_options(TracingOptions {
        default_level: if matches!(cli.command, Commands::Bin { .. } | Commands::Run { .. }) {
            LevelFilter::WARN
        } else if matches!(cli.command, Commands::Completions { .. }) {
            LevelFilter::OFF
        } else {
            LevelFilter::INFO
        },
        filter_modules: modules,
        intercept_log: false,
        log_env: "STARBASE_LOG".into(),
        // test_env: "PROTO_TEST".into(),
        ..TracingOptions::default()
    });

    let mut args = env::args_os().collect::<Vec<_>>();

    debug!(
        bin = ?args.remove(0),
        args = ?args,
        shim = env::var("PROTO_SHIM_NAME").ok(),
        shim_bin = env::var("PROTO_SHIM_PATH").ok(),
        pid = std::process::id(),
        "Running proto v{}",
        version
    );

    let mut app = App::new();
    app.startup(systems::detect_proto_env);
    app.analyze(systems::load_proto_configs);
    app.analyze(systems::remove_old_bins);

    if !matches!(
        cli.command,
        Commands::Bin(_)
            | Commands::Completions(_)
            | Commands::Run(_)
            | Commands::Setup(_)
            | Commands::Upgrade
    ) {
        app.execute(systems::check_for_new_version);
    }

    match cli.command {
        Commands::Alias(args) => app.execute_with_args(commands::alias, args),
        Commands::Bin(args) => app.execute_with_args(commands::bin, args),
        Commands::Clean(args) => app.execute_with_args(commands::clean, args),
        Commands::Completions(args) => app.execute_with_args(commands::completions, args),
        Commands::Debug { command } => match command {
            DebugCommands::Config(args) => app.execute_with_args(commands::debug::config, args),
            DebugCommands::Env => app.execute(commands::debug::env),
        },
        Commands::Install(args) => app.execute_with_args(commands::install, args),
        Commands::InstallGlobal(args) => app.execute_with_args(commands::install_global, args),
        Commands::List(args) => app.execute_with_args(commands::list, args),
        Commands::ListGlobal(args) => app.execute_with_args(commands::list_global, args),
        Commands::ListRemote(args) => app.execute_with_args(commands::list_remote, args),
        Commands::Migrate(args) => app.execute_with_args(commands::migrate, args),
        Commands::Outdated(args) => app.execute_with_args(commands::outdated, args),
        Commands::Pin(args) => app.execute_with_args(commands::pin, args),
        Commands::Regen(args) => app.execute_with_args(commands::regen, args),
        Commands::Run(args) => app.execute_with_args(commands::run, args),
        Commands::Setup(args) => app.execute_with_args(commands::setup, args),
        Commands::Tool { command } => match command {
            ToolCommands::Add(args) => app.execute_with_args(commands::tool::add, args),
            ToolCommands::Info(args) => app.execute_with_args(commands::tool::info, args),
            ToolCommands::List(args) => app.execute_with_args(commands::tool::list, args),
            ToolCommands::Remove(args) => app.execute_with_args(commands::tool::remove, args),
        },
        Commands::Unalias(args) => app.execute_with_args(commands::unalias, args),
        Commands::Uninstall(args) => app.execute_with_args(commands::uninstall, args),
        Commands::UninstallGlobal(args) => app.execute_with_args(commands::uninstall_global, args),
        Commands::Upgrade => app.execute(commands::upgrade),
        Commands::Use => app.execute(commands::install_all),
    };

    app.run().await?;

    Ok(())
}
