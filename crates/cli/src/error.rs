use miette::Diagnostic;
use proto_core::PROTO_CONFIG_NAME;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

// Convention: <command><action><component>

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoCliError {
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
}
