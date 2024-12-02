use miette::Diagnostic;
use proto_core::PROTO_CONFIG_NAME;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoCliError {
    #[diagnostic(code(proto::cli::invalid_alias))]
    #[error("Invalid alias name {}. Use alpha-numeric words instead.", .alias.style(Style::Id))]
    InvalidAliasName { alias: String },

    #[diagnostic(code(proto::cli::missing_tools_config))]
    #[error(
        "No {} has been found in current directory. Attempted to find at {}.",
        PROTO_CONFIG_NAME.style(Style::File),
        .path.style(Style::Path),
    )]
    MissingToolsConfigInCwd { path: PathBuf },

    #[diagnostic(code(proto::cli::missing_alternate_binary))]
    #[error(
        "Unable to run, alternate binary {} does not exist. Attempted to find at {}.",
        .bin.style(Style::File),
        .path.style(Style::Path),
    )]
    MissingRunAltBin { bin: String, path: PathBuf },

    #[diagnostic(code(proto::cli::no_configured_tools))]
    #[error("No tools have been configured in {}.", PROTO_CONFIG_NAME.style(Style::File))]
    NoConfiguredTools,

    #[diagnostic(code(proto::cli::no_mapped_alias))]
    #[error("Cannot map an alias to itself.")]
    NoMatchingAliasToVersion,

    #[diagnostic(code(proto::cli::no_self_upgrade))]
    #[error(
        "Self upgrading {} is not supported in proto, as it conflicts with proto's managed inventory.\nUse {} instead to upgrade to the latest version.",
        .tool,
        .command.style(Style::Shell)
    )]
    NoSelfUpgrade { command: String, tool: String },

    #[diagnostic(
        code(proto::cli::requirements_not_met),
        help("Try configuring a version of the required tool in .prototools")
    )]
    #[error(
        "{} requires {} to function correctly, but it has not been installed.",
        .tool,
        .requires.style(Style::Id)
    )]
    ToolRequiresNotMet { tool: String, requires: String },

    #[diagnostic(code(proto::cli::upgrade_failed))]
    #[error("Failed to upgrade proto, {} could not be located after download!", .bin.style(Style::Shell))]
    UpgradeFailed { bin: String },

    #[diagnostic(code(proto::cli::offline))]
    #[error("Upgrading proto requires an internet connection!")]
    UpgradeRequiresInternet,

    #[diagnostic(code(proto::cli::running_process))]
    #[error("Unable to upgrade as an instance of proto is currently running with the process ID {}. Please stop this process then try again.", .pid.to_string().style(Style::Symbol))]
    CannotUpgradeProtoRunning { pid: u32 },

    #[diagnostic(code(proto::cli::unknown_migration))]
    #[error("Unknown migration operation {}.", .op.style(Style::Symbol))]
    UnknownMigration { op: String },
}
