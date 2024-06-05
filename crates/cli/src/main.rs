mod app;
mod commands;
mod error;
mod helpers;
mod printer;
mod session;
mod shell;
mod systems;
mod telemetry;

use app::{App as CLI, Commands, DebugCommands, PluginCommands};
use clap::Parser;
use miette::IntoDiagnostic;
use session::ProtoSession;
use starbase::{tracing::TracingOptions, App, MainResult};
use starbase_utils::string_vec;
use std::env;
use tracing::{debug, metadata::LevelFilter};

fn get_tracing_modules() -> Vec<String> {
    let mut modules = string_vec!["proto", "schematic", "starbase", "warpgate"];

    if env::var("PROTO_WASM_LOG").is_ok() {
        modules.push("extism".into());
    } else {
        modules.push("extism::pdk".into());
    }

    modules
}

#[tokio::main]
async fn main() -> MainResult {
    let mut app = App::default();
    app.setup_diagnostics();

    let cli = CLI::try_parse().into_diagnostic()?;

    let _guard = app.setup_tracing(TracingOptions {
        default_level: if matches!(cli.command, Commands::Bin { .. } | Commands::Run { .. }) {
            LevelFilter::WARN
        } else if matches!(cli.command, Commands::Completions { .. }) {
            LevelFilter::OFF
        } else {
            LevelFilter::INFO
        },
        dump_trace: cli.dump && !matches!(cli.command, Commands::Run { .. }),
        filter_modules: get_tracing_modules(),
        intercept_log: false,
        log_env: "STARBASE_LOG".into(),
        // test_env: "PROTO_TEST".into(),
        ..TracingOptions::default()
    });

    let mut session = ProtoSession::new(cli);
    let mut args = env::args_os().collect::<Vec<_>>();

    debug!(
        bin = ?args.remove(0),
        args = ?args,
        shim = env::var("PROTO_SHIM_NAME").ok(),
        shim_bin = env::var("PROTO_SHIM_PATH").ok(),
        pid = std::process::id(),
        "Running proto v{}",
        session.cli_version
    );

    app.run(&mut session, |session| match session.cli.command {
        Commands::Alias(args) => commands::alias(session, args),
        Commands::Bin(args) => commands::bin(session, args),
        Commands::Clean(args) => commands::clean(session, args),
        Commands::Completions(args) => commands::completions(session, args),
        Commands::Debug { command } => match command {
            DebugCommands::Config(args) => commands::debug::config(session, args),
            DebugCommands::Env => commands::debug::env(session),
        },
        Commands::Install(args) => commands::install(session, args),
        Commands::List(args) => commands::list(session, args),
        Commands::ListRemote(args) => commands::list_remote(session, args),
        Commands::Migrate(args) => commands::migrate(session, args),
        Commands::Outdated(args) => commands::outdated(session, args),
        Commands::Pin(args) => commands::pin(session, args),
        Commands::Plugin { command } => match command {
            PluginCommands::Add(args) => commands::plugin::add(session, args),
            PluginCommands::Info(args) => commands::plugin::info(session, args),
            PluginCommands::List(args) => commands::plugin::list(session, args),
            PluginCommands::Remove(args) => commands::plugin::remove(session, args),
            PluginCommands::Search(args) => commands::plugin::search(session, args),
        },
        Commands::Regen(args) => commands::regen(session, args),
        Commands::Run(args) => commands::run(session, args),
        Commands::Setup(args) => commands::setup(session, args),
        Commands::Status(args) => commands::status(session, args),
        Commands::Unalias(args) => commands::unalias(session, args),
        Commands::Uninstall(args) => commands::uninstall(session, args),
        Commands::Unpin(args) => commands::unpin(session, args),
        Commands::Upgrade => commands::upgrade(session),
        Commands::Use => commands::install_all(session),
    })
    .await?;

    Ok(())
}
