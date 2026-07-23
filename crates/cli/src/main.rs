mod app;
mod commands;
mod components;
mod error;
mod helpers;
mod mcp;
mod session;
mod shell;
mod systems;
mod telemetry;
mod utils;
mod workflows;

use app::{App as CLI, Commands, DebugCommands, PluginCommands, StdoutOwner};
use clap::Parser;
use proto_core::reporter::ReporterFormat;
use session::ProtoSession;
use starbase::{
    App, MainResult,
    tracing::{LogLevel, OtelOptions, TracingOptions},
};
use starbase_utils::{envx, string_vec};
use std::env;
use std::process::ExitCode;
use tracing::debug;

fn get_tracing_modules() -> Vec<String> {
    let mut modules = string_vec!["proto", "schematic", "starbase", "warpgate"];

    if envx::bool_var("PROTO_DEBUG_WASM") || envx::bool_var("PROTO_WASM_LOG") {
        modules.push("extism".into());
    } else {
        modules.push("extism::pdk".into());
    }

    modules
}

async fn async_main() -> MainResult {
    let cli = CLI::parse();
    cli.setup_env_vars();

    let app = App::default();
    app.setup_diagnostics();

    let stdout_owner = cli.stdout_owner();
    let is_exec_command = matches!(
        &cli.command,
        Commands::Exec(_) | Commands::Run(_) | Commands::Shell(_)
    );

    let _guard = app.setup_tracing(TracingOptions {
        default_level: if is_exec_command || matches!(cli.command, Commands::Bin { .. }) {
            LogLevel::Warn
        } else if matches!(stdout_owner, StdoutOwner::CompletionCode) {
            LogLevel::Off
        } else {
            LogLevel::Info
        },
        dump_trace: cli.dump && !is_exec_command,
        filter_modules: get_tracing_modules(),
        log_env: "PROTO_APP_LOG".into(),
        log_file: cli.log_file.clone(),
        ndjson: cli.reporter_format() == ReporterFormat::Ndjson,
        otel: OtelOptions {
            enabled: cli.otel,
            logs_enabled: cli.otel_logs,
            service_name: cli.otel_service_name.clone(),
            ..Default::default()
        },
        show_spans: cli.log.is_verbose(),
        // test_env: "PROTO_TEST".into(),
        ..TracingOptions::default()
    })?;

    let session = ProtoSession::new(cli);
    let mut args = env::args_os().collect::<Vec<_>>();

    debug!(
        exe = ?args.remove(0),
        args = ?args,
        shim = env::var("PROTO_SHIM_NAME").ok(),
        shim_exe = env::var("PROTO_SHIM_PATH").ok(),
        pid = std::process::id(),
        "Running proto v{}",
        session.cli_version
    );

    let mut outcome = app
        .run(session.clone(), |session: ProtoSession| async {
            match session.cli.command.clone() {
                Commands::Activate(args) => commands::activate(session, args).await,
                Commands::Alias(args) => commands::alias(session, args).await,
                Commands::Bin(args) => commands::bin(session, args).await,
                Commands::Clean(args) => commands::clean(session, args).await,
                Commands::Completions(args) => commands::completions(session, args).await,
                Commands::Debug { command } => match command {
                    DebugCommands::Config(args) => commands::debug::config(session, args).await,
                    DebugCommands::Env(args) => commands::debug::env(session, args).await,
                },
                Commands::Diagnose(args) => commands::diagnose(session, args).await,
                Commands::Exec(args) => commands::exec(session, args).await,
                Commands::Install(args) => commands::install(session, args).await,
                Commands::Mcp(args) => commands::mcp(session, args).await,
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
                Commands::Shell(args) => commands::shell(session, args).await,
                Commands::Status(args) => commands::status(session, args).await,
                Commands::Unalias(args) => commands::unalias(session, args).await,
                Commands::Uninstall(args) => commands::uninstall(session, args).await,
                Commands::Unpin(args) => commands::unpin(session, args).await,
                Commands::Upgrade(args) => commands::upgrade(session, args).await,
                Commands::Versions(args) => commands::versions(session, args).await,
            }
        })
        .await;

    if let Some(error) = outcome.error {
        // Keep NDJSON errors machine-readable without violating stdout
        // ownership. The reporter stream is configured by ProtoSession.
        if session.cli.reporter_format() == ReporterFormat::Ndjson {
            session
                .console
                .main_error(error.to_string(), error.code().map(|code| code.to_string()))?;

            if outcome.exit_code == 0 {
                outcome.exit_code = 1;
            }
        }
        // Otherwise bubble up the error so that miette renders
        // it nicely for the user using the fancy output!
        else {
            return Err(error);
        }
    }

    Ok(ExitCode::from(outcome.exit_code))
}

fn main() -> MainResult {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .name("proto")
        .thread_name("proto-worker")
        // We need more stack space to handle WASM plugins.
        // The default stack size is 2MB, but we increase it to 8MB here.
        .thread_stack_size(8 * 1024 * 1024)
        .build()
        .unwrap()
        .block_on(async_main())
}
