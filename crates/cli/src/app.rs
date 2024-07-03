use crate::commands::{
    debug::DebugConfigArgs,
    plugin::{AddPluginArgs, InfoPluginArgs, ListPluginsArgs, RemovePluginArgs, SearchPluginArgs},
    ActivateArgs, AliasArgs, BinArgs, CleanArgs, CompletionsArgs, DiagnoseArgs, InstallArgs,
    ListArgs, ListRemoteArgs, MigrateArgs, OutdatedArgs, PinArgs, RegenArgs, RunArgs, SetupArgs,
    StatusArgs, UnaliasArgs, UninstallArgs, UnpinArgs,
};
use clap::builder::styling::{Color, Style, Styles};
use clap::{Parser, Subcommand, ValueEnum};
use starbase_styles::color::Color as ColorType;
use std::{
    env,
    fmt::{Display, Error, Formatter},
};

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "{}",
            match self {
                LogLevel::Off => "off",
                LogLevel::Error => "error",
                LogLevel::Warn => "warn",
                LogLevel::Info => "info",
                LogLevel::Debug => "debug",
                LogLevel::Trace => "trace",
            }
        )?;

        Ok(())
    }
}

fn fg(ty: ColorType) -> Style {
    Style::new().fg_color(Some(Color::from(ty as u8)))
}

fn create_styles() -> Styles {
    Styles::default()
        .error(fg(ColorType::Red))
        .header(Style::new().bold())
        .invalid(fg(ColorType::Yellow))
        .literal(fg(ColorType::Pink)) // args, options, etc
        .placeholder(fg(ColorType::GrayLight))
        .usage(fg(ColorType::Purple).bold())
        .valid(fg(ColorType::Green))
}

#[derive(Clone, Debug, Parser)]
#[command(
    name = "proto",
    version,
    about,
    long_about = None,
    disable_help_subcommand = true,
    propagate_version = true,
    next_line_help = false,
    styles = create_styles()
)]
pub struct App {
    #[arg(
        long,
        global = true,
        env = "PROTO_DUMP",
        help = "Dump a trace profile to the working directory"
    )]
    pub dump: bool,

    #[arg(
        value_enum,
        long,
        global = true,
        env = "PROTO_LOG",
        help = "Lowest log level to output"
    )]
    pub log: Option<LogLevel>,

    #[command(subcommand)]
    pub command: Commands,
}

impl App {
    pub fn setup_env_vars(&self) {
        let version = env!("CARGO_PKG_VERSION");

        if let Some(level) = &self.log {
            env::set_var("PROTO_APP_LOG", level.to_string());
        } else if let Ok(level) = env::var("PROTO_LOG") {
            env::set_var("PROTO_APP_LOG", level);
        }

        env::set_var("PROTO_VERSION", version);

        if let Ok(value) = env::var("PROTO_DEBUG_COMMAND") {
            env::set_var("WARPGATE_DEBUG_COMMAND", value);
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    #[command(
        name = "activate",
        about = "Activate proto for the current shell session by prepending tool directories to PATH and setting environment variables.",
        long_about = "Activate proto for the current shell session by prepending tool directories to PATH and setting environment variables.\n\nThis should be ran within your shell profile.\nLearn more: https://moonrepo.dev/docs/proto/workflows"
    )]
    Activate(ActivateArgs),

    #[command(
        alias = "a",
        name = "alias",
        about = "Add an alias to a tool.",
        long_about = "Add an alias to a tool, that maps to a specific version, or another alias."
    )]
    Alias(AliasArgs),

    #[command(
        name = "bin",
        about = "Display the absolute path to a tools executable.",
        long_about = "Display the absolute path to a tools executable. If no version is provided,\nit will be detected from the current environment."
    )]
    Bin(BinArgs),

    #[command(
        name = "clean",
        about = "Clean the ~/.proto directory by removing stale tools, plugins, and files."
    )]
    Clean(CleanArgs),

    #[command(
        name = "completions",
        about = "Generate command completions for your current shell."
    )]
    Completions(CompletionsArgs),

    #[command(name = "debug", about = "Debug the current proto environment.")]
    Debug {
        #[command(subcommand)]
        command: DebugCommands,
    },

    #[command(
        alias = "doctor",
        name = "diagnose",
        about = "Diagnose potential issues with your proto installation."
    )]
    Diagnose(DiagnoseArgs),

    #[command(
        alias = "i",
        name = "install",
        about = "Download and install a tool.",
        long_about = "Download and install a tool by version into ~/.proto/tools."
    )]
    Install(InstallArgs),

    #[command(
        alias = "ls",
        name = "list",
        about = "List installed versions for a tool."
    )]
    List(ListArgs),

    #[command(
        alias = "lsr",
        name = "list-remote",
        about = "List available versions for a tool.",
        long_about = "List available versions by resolving versions from the tool's remote release manifest."
    )]
    ListRemote(ListRemoteArgs),

    #[command(
        name = "migrate",
        about = "Migrate breaking changes for the proto installation."
    )]
    Migrate(MigrateArgs),

    #[command(
        alias = "o",
        name = "outdated",
        about = "Check if configured tool versions are out of date."
    )]
    Outdated(OutdatedArgs),

    #[command(
        alias = "p",
        name = "pin",
        about = "Pin a global or local version of a tool.",
        long_about = "Pin a version of a tool globally to ~/.proto/.prototools, or locally to ./.prototools."
    )]
    Pin(PinArgs),

    #[command(
        alias = "tool", // Deprecated
        name = "plugin",
        about = "Operations for managing tool plugins."
    )]
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },

    #[command(name = "regen", about = "Regenerate shims and optionally relink bins.")]
    Regen(RegenArgs),

    #[command(
        alias = "r",
        name = "run",
        about = "Run a tool after detecting a version from the environment.",
        long_about = "Run a tool after detecting a version from the environment. In order of priority,\na version will be resolved from a provided CLI argument, a PROTO_VERSION environment variable,\na local version file (.prototools), and lastly a global version file (~/.proto/tools).\n\nIf no version can be found, the program will exit with an error."
    )]
    Run(RunArgs),

    #[command(
        name = "setup",
        about = "Setup proto for your current shell by injecting exports and updating PATH."
    )]
    Setup(SetupArgs),

    #[command(
        name = "status",
        about = "List all configured tools and their current installation status."
    )]
    Status(StatusArgs),

    #[command(alias = "ua", name = "unalias", about = "Remove an alias from a tool.")]
    Unalias(UnaliasArgs),

    #[command(
        alias = "ui",
        name = "uninstall",
        about = "Uninstall a tool.",
        long_about = "Uninstall a tool and remove the installation from ~/.proto/tools."
    )]
    Uninstall(UninstallArgs),

    #[command(
        alias = "uv",
        name = "unpin",
        about = "Unpin a global or local version of a tool."
    )]
    Unpin(UnpinArgs),

    #[command(
        alias = "up",
        name = "upgrade",
        about = "Upgrade proto to the latest version."
    )]
    Upgrade,

    #[command(
        alias = "u",
        name = "use",
        about = "Download and install all tools from loaded .prototools."
    )]
    Use,
}

#[derive(Clone, Debug, Subcommand)]
pub enum DebugCommands {
    #[command(
        name = "config",
        about = "Debug all loaded .prototools config's for the current directory."
    )]
    Config(DebugConfigArgs),

    #[command(name = "env", about = "Debug the current proto environment and store.")]
    Env,
}

#[derive(Clone, Debug, Subcommand)]
pub enum PluginCommands {
    #[command(
        name = "add",
        about = "Add a plugin to manage a tool.",
        long_about = "Add a plugin to the local ./.prototools config, or global ~/.proto/.prototools config."
    )]
    Add(AddPluginArgs),

    #[command(
        name = "info",
        about = "Display information about an installed plugin and its inventory."
    )]
    Info(InfoPluginArgs),

    #[command(
        name = "list",
        about = "List all configured and built-in plugins, and optionally include inventory."
    )]
    List(ListPluginsArgs),

    #[command(
        name = "remove",
        about = "Remove a plugin and unmanage a tool.",
        long_about = "Remove a plugin from the local ./.prototools config, or global ~/.proto/.prototools config."
    )]
    Remove(RemovePluginArgs),

    #[command(
        name = "search",
        about = "Search for available plugins provided by the community."
    )]
    Search(SearchPluginArgs),
}
