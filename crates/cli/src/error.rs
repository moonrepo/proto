use miette::Diagnostic;
use proto_core::flow::resolve::ProtoResolveError;
use proto_core::flow::setup::ProtoSetupError;
use proto_core::layout::ProtoLayoutError;
use proto_core::warpgate::WarpgatePluginError;
use proto_core::{PROTO_CONFIG_NAME, ProtoConfigError, ProtoIdError};
use starbase_console::ConsoleError;
use starbase_shell::ShellError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use std::path::PathBuf;
use thiserror::Error;

// Convention: <command><action><component>

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoCliError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Config(#[from] Box<ProtoConfigError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Console(#[from] Box<ConsoleError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[error(transparent)]
    Http(#[from] Box<reqwest::Error>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Id(#[from] Box<ProtoIdError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Layout(#[from] Box<ProtoLayoutError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Resolve(#[from] Box<ProtoResolveError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Setup(#[from] Box<ProtoSetupError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Shell(#[from] Box<ShellError>),

    #[diagnostic(code(proto::no_configured_tools))]
    #[error("No tools have been configured in {}.", PROTO_CONFIG_NAME.style(Style::File))]
    NoConfiguredTools,

    #[diagnostic(code(proto::missing_tools_config))]
    #[error(
        "No {} has been found in current directory. Attempted to find at {}.",
        PROTO_CONFIG_NAME.style(Style::File),
        .path.style(Style::Path),
    )]
    MissingToolsConfigInCwd { path: PathBuf },

    // ALIAS
    #[diagnostic(code(proto::commands::alias::invalid))]
    #[error("Invalid alias name {}. Use alpha-numeric words instead.", .alias.style(Style::Id))]
    AliasInvalidName { alias: String },

    #[diagnostic(code(proto::commands::alias::no_mapping))]
    #[error("Cannot map an alias to itself.")]
    AliasNoMatchingToVersion,

    // EXEC
    #[diagnostic(code(proto::commands::exec::missing_command))]
    #[error(
        "A command is required for execution. The command and its arguments can be passed after {}.",
        "--".style(Style::Shell)
    )]
    ExecMissingCommand,

    // INSTALL
    #[diagnostic(
        code(proto::commands::install::requirements_not_met),
        help("Try configuring a version of the required tool in .prototools")
    )]
    #[error(
        "{} requires {} to function correctly, but it has not been installed.",
        .tool,
        .requires.style(Style::Id)
    )]
    InstallRequirementsNotMet { tool: String, requires: String },

    // MIGRATE
    #[diagnostic(code(proto::commands::migrate::unknown))]
    #[error("Unknown migration operation {}.", .op.style(Style::Symbol))]
    MigrateUnknownOperation { op: String },

    // RUN
    #[diagnostic(code(proto::commands::run::missing_alternate_binary))]
    #[error(
        "Unable to run, alternate binary {} does not exist. Attempted to find at {}.",
        .bin.style(Style::File),
        .path.style(Style::Path),
    )]
    RunMissingAltBin { bin: String, path: PathBuf },

    #[diagnostic(code(proto::commands::run::missing_tool))]
    #[error(
        "This project requires {tool} {}, but this version has not been installed. Install it with {}, or enable the {} setting to automatically install missing versions!",
        .version.style(Style::Hash),
        .command.style(Style::Shell),
        "auto-install".style(Style::Property),
    )]
    RunMissingTool {
        tool: String,
        version: String,
        command: String,
    },

    #[diagnostic(code(proto::commands::run::missing_tool))]
    #[error(
        "This project requires {tool} {} (detected from {}), but this version has not been installed. Install it with {}, or enable the {} setting to automatically install missing versions!",
        .version.style(Style::Hash),
        .path.style(Style::Path),
        .command.style(Style::Shell),
        "auto-install".style(Style::Property),
    )]
    RunMissingToolWithSource {
        tool: String,
        version: String,
        command: String,
        path: PathBuf,
    },

    #[diagnostic(code(proto::commands::run::no_self_upgrade))]
    #[error(
        "Self upgrading {} is not supported in proto, as it conflicts with proto's managed inventory.\nUse {} instead to upgrade to the latest version.",
        .tool,
        .command.style(Style::Shell)
    )]
    RunNoSelfUpgrade { command: String, tool: String },

    // UPGRADE
    #[diagnostic(code(proto::commands::upgrade::failed))]
    #[error("Failed to upgrade proto, {} binary could not be located after download!", .bin.style(Style::Shell))]
    UpgradeFailed { bin: String },

    #[diagnostic(code(proto::commands::upgrade::offline))]
    #[error("Upgrading proto requires an internet connection!")]
    UpgradeRequiresInternet,

    #[diagnostic(
        code(proto::commands::upgrade::fetch_version_failed),
        help = "Can you connect to github.com?"
    )]
    #[error("Failed to fetch the latest available version.")]
    FailedToFetchVersion,
}

impl From<ProtoConfigError> for ProtoCliError {
    fn from(e: ProtoConfigError) -> ProtoCliError {
        ProtoCliError::Config(Box::new(e))
    }
}

impl From<ConsoleError> for ProtoCliError {
    fn from(e: ConsoleError) -> ProtoCliError {
        ProtoCliError::Console(Box::new(e))
    }
}

impl From<FsError> for ProtoCliError {
    fn from(e: FsError) -> ProtoCliError {
        ProtoCliError::Fs(Box::new(e))
    }
}

impl From<ProtoIdError> for ProtoCliError {
    fn from(e: ProtoIdError) -> ProtoCliError {
        ProtoCliError::Id(Box::new(e))
    }
}

impl From<reqwest::Error> for ProtoCliError {
    fn from(e: reqwest::Error) -> ProtoCliError {
        ProtoCliError::Http(Box::new(e))
    }
}

impl From<ProtoLayoutError> for ProtoCliError {
    fn from(e: ProtoLayoutError) -> ProtoCliError {
        ProtoCliError::Layout(Box::new(e))
    }
}

impl From<WarpgatePluginError> for ProtoCliError {
    fn from(e: WarpgatePluginError) -> ProtoCliError {
        ProtoCliError::Plugin(Box::new(e))
    }
}

impl From<ProtoResolveError> for ProtoCliError {
    fn from(e: ProtoResolveError) -> ProtoCliError {
        ProtoCliError::Resolve(Box::new(e))
    }
}

impl From<ProtoSetupError> for ProtoCliError {
    fn from(e: ProtoSetupError) -> ProtoCliError {
        ProtoCliError::Setup(Box::new(e))
    }
}

impl From<ShellError> for ProtoCliError {
    fn from(e: ShellError) -> ProtoCliError {
        ProtoCliError::Shell(Box::new(e))
    }
}
