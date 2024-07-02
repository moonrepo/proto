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
use session::ProtoSession;
use starbase::{
    tracing::{LogLevel, TracingOptions},
    App, MainResult,
};
use starbase_utils::{env::bool_var, string_vec};
use std::env;
use tracing::debug;

fn get_tracing_modules() -> Vec<String> {
    let mut modules = string_vec!["proto", "schematic", "starbase", "warpgate"];

    if bool_var("PROTO_WASM_LOG") {
        modules.push("extism".into());
    } else {
        modules.push("extism::pdk".into());
    }

    modules
}

#[tokio::main]
async fn main() -> MainResult {
    sigpipe::reset();

    let cli = CLI::parse();
    cli.setup_env_vars();

    let app = App::default();
    app.setup_diagnostics();

    let _guard = app.setup_tracing(TracingOptions {
        default_level: if matches!(cli.command, Commands::Bin { .. } | Commands::Run { .. }) {
            LogLevel::Warn
        } else if matches!(cli.command, Commands::Completions { .. }) {
            LogLevel::Off
        } else {
            LogLevel::Info
        },
        dump_trace: cli.dump && !matches!(cli.command, Commands::Run { .. }),
        filter_modules: get_tracing_modules(),
        log_env: "PROTO_APP_LOG".into(),
        // test_env: "PROTO_TEST".into(),
        ..TracingOptions::default()
    });

    let session = ProtoSession::new(cli);
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

    app.run(session, |session| async {
        match session.cli.command.clone() {
            Commands::Activate(args) => commands::activate(session, args).await,
            Commands::Alias(args) => commands::alias(session, args).await,
            Commands::Bin(args) => commands::bin(session, args).await,
            Commands::Clean(args) => commands::clean(session, args).await,
            Commands::Completions(args) => commands::completions(session, args).await,
            Commands::Debug { command } => match command {
                DebugCommands::Config(args) => commands::debug::config(session, args).await,
                DebugCommands::Env => commands::debug::env(session).await,
            },
            Commands::Diagnose(args) => commands::diagnose(session, args).await,
            Commands::Install(args) => commands::install(session, args).await,
            Commands::List(args) => commands::list(session, args).await,
            Commands::ListRemote(args) => commands::list_remote(session, args).await,
            Commands::Migrate(args) => commands::migrate(session, args).await,
            Commands::Outdated(args) => commands::outdated(session, args).await,
            Commands::Pin(args) => commands::pin(session, args).await,
            Commands::Plugin { command } => match command {
                PluginCommands::Add(args) => commands::plugin::add(session, args).await,
                PluginCommands::Info(args) => commands::plugin::info(session, args).await,
                PluginCommands::List(args) => commands::plugin::list(session, args).await,
                PluginCommands::Remove(args) => commands::plugin::remove(session, args).await,
                PluginCommands::Search(args) => commands::plugin::search(session, args).await,
            },
            Commands::Regen(args) => commands::regen(session, args).await,
            Commands::Run(args) => commands::run(session, args).await,
            Commands::Setup(args) => commands::setup(session, args).await,
            Commands::Status(args) => commands::status(session, args).await,
            Commands::Unalias(args) => commands::unalias(session, args).await,
            Commands::Uninstall(args) => commands::uninstall(session, args).await,
            Commands::Unpin(args) => commands::unpin(session, args).await,
            Commands::Upgrade => commands::upgrade(session).await,
            Commands::Use(args) => commands::install_all(session, args).await,
        }
    })
    .await?;

    Ok(())
}
