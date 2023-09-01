use crate::commands::{
    AddPluginArgs, AliasArgs, BinArgs, CleanArgs, CompletionsArgs, GlobalArgs, InstallArgs,
    InstallGlobalArgs, ListArgs, ListGlobalArgs, ListRemoteArgs, LocalArgs, PluginsArgs, RunArgs,
    SetupArgs, UnaliasArgs, UninstallArgs, UninstallGlobalArgs,
};
use clap::{Parser, Subcommand, ValueEnum};
use std::fmt::{Display, Error, Formatter};

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

#[derive(Debug, Parser)]
#[command(
    name = "proto",
    version,
    about,
    long_about = None,
    disable_colored_help = true,
    disable_help_subcommand = true,
    propagate_version = true,
    next_line_help = false,
    rename_all = "camelCase"
)]
pub struct App {
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

#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    #[command(
        name = "ap",
        name = "add-plugin",
        about = "Add a plugin to utilize.",
        long_about = "Add a plugin to utilize, either by inserting into the local .prototools config, or global ~/.proto/config.toml config."
    )]
    AddPlugin(AddPluginArgs),

    #[command(
        name = "a",
        name = "alias",
        about = "Add an alias to a tool.",
        long_about = "Add an alias to a tool, that maps to a specific version, or another alias."
    )]
    Alias(AliasArgs),

    #[command(
        name = "bin",
        about = "Display the absolute path to a tools binary.",
        long_about = "Display the absolute path to a tools binary. If no version is provided,\nit will detected from the current environment."
    )]
    Bin(BinArgs),

    #[command(
        name = "clean",
        about = "Clean the ~/.proto directory by removing stale and old tools."
    )]
    Clean(CleanArgs),

    #[command(
        name = "completions",
        about = "Generate command completions for your current shell."
    )]
    Completions(CompletionsArgs),

    #[command(
        alias = "i",
        name = "install",
        about = "Download and install a tool.",
        long_about = "Download and install a tool by unpacking the archive to ~/.proto/tools."
    )]
    Install(InstallArgs),

    #[command(
        alias = "ig",
        name = "install-global",
        about = "Install a global dependency for the specified tool.",
        long_about = "Install a global dependency for the specified tool. Depending on the tool, the dependency will either be installed to ~/.proto/tools/<tool>/globals or ~/<tool>."
    )]
    InstallGlobal(InstallGlobalArgs),

    #[command(
        name = "global",
        about = "Set the global default version of a tool.",
        long_about = "Set the global default version of a tool. This will pin the version in the ~/.proto/tools installation directory."
    )]
    Global(GlobalArgs),

    #[command(
        name = "list",
        alias = "ls",
        about = "List installed versions.",
        long_about = "List installed versions by scanning the ~/.proto/tools directory for possible versions."
    )]
    List(ListArgs),

    #[command(
        name = "list-global",
        alias = "lsg",
        about = "List installed globals.",
        long_about = "List installed globals by scanning the global bins installation directory. Will return the canonical source path."
    )]
    ListGlobal(ListGlobalArgs),

    #[command(
        name = "list-remote",
        alias = "lsr",
        about = "List available versions.",
        long_about = "List available versions by resolving versions from the tool's remote release manifest."
    )]
    ListRemote(ListRemoteArgs),

    #[command(
        name = "local",
        about = "Set the local version of a tool.",
        long_about = "Set the local version of a tool. This will create a .prototools file (if it does not exist)\nin the current working directory with the appropriate tool and version."
    )]
    Local(LocalArgs),

    #[command(name = "plugins", about = "List all active and configured plugins.")]
    Plugins(PluginsArgs),

    #[command(
        alias = "r",
        name = "run",
        about = "Run a tool after detecting a version from the environment.",
        long_about = "Run a tool after detecting a version from the environment. In order of priority,\na version will be resolved from a provided CLI argument, a PROTO_VERSION environment variable,\na local version file (.prototools), and lastly a global version file (~/.proto/tools/version).\n\nIf no version can be found, the program will exit with an error."
    )]
    Run(RunArgs),

    #[command(name = "setup", about = "Setup proto for your current shell.")]
    Setup(SetupArgs),

    #[command(name = "ua", name = "unalias", about = "Remove an alias from a tool.")]
    Unalias(UnaliasArgs),

    #[command(
        name = "ui",
        name = "uninstall",
        about = "Uninstall a tool.",
        long_about = "Uninstall a tool and remove the installation from ~/.proto/tools."
    )]
    Uninstall(UninstallArgs),

    #[command(
        alias = "ug",
        name = "uninstall-global",
        about = "Uninstall a global dependency from the specified tool."
    )]
    UninstallGlobal(UninstallGlobalArgs),

    #[command(
        alias = "up",
        name = "upgrade",
        about = "Upgrade proto to the latest version."
    )]
    Upgrade,

    #[command(
        alias = "u",
        name = "use",
        about = "Download and install all tools from the closest .prototools."
    )]
    Use,
}
