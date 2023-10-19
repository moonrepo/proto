use crate::commands::{
    AddPluginArgs, AliasArgs, BinArgs, CleanArgs, CompletionsArgs, InstallArgs, InstallGlobalArgs,
    ListArgs, ListGlobalArgs, ListRemoteArgs, MigrateArgs, OutdatedArgs, PinArgs, PluginsArgs,
    RemovePluginArgs, RunArgs, SetupArgs, ToolsArgs, UnaliasArgs, UninstallArgs,
    UninstallGlobalArgs,
};
use clap::builder::styling::{Color, Style, Styles};
use clap::{Parser, Subcommand, ValueEnum};
use starbase_styles::color::Color as ColorType;
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

#[derive(Debug, Parser)]
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
        alias = "ap",
        name = "add-plugin",
        about = "Add a plugin.",
        long_about = "Add a plugin to the local .prototools config, or global ~/.proto/config.toml config."
    )]
    AddPlugin(AddPluginArgs),

    #[command(
        alias = "a",
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
        about = "Clean the ~/.proto directory by removing stale tools, plugins, and files."
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
        alias = "ls",
        name = "list",
        about = "List installed versions.",
        long_about = "List installed versions by scanning the ~/.proto/tools directory for possible versions."
    )]
    List(ListArgs),

    #[command(
        alias = "lsg",
        name = "list-global",
        about = "List installed globals.",
        long_about = "List installed globals by scanning the global bins installation directory. Will return the canonical source path."
    )]
    ListGlobal(ListGlobalArgs),

    #[command(
        alias = "lsr",
        name = "list-remote",
        about = "List available versions.",
        long_about = "List available versions by resolving versions from the tool's remote release manifest."
    )]
    ListRemote(ListRemoteArgs),

    #[command(
        name = "migrate",
        about = "Migrate breaking changes for the proto installation."
    )]
    Migrate(MigrateArgs),

    #[command(
        name = "outdated",
        about = "Check if configured tool versions are out of date."
    )]
    Outdated(OutdatedArgs),

    #[command(
        alias = "p",
        name = "pin",
        about = "Pin a default global or local version of a tool.",
        long_about = "Pin a default version of a tool globally to ~/.proto/tools, or locally to .prototools (in the current working directory)."
    )]
    Pin(PinArgs),

    #[command(name = "plugins", about = "List all active and configured plugins.")]
    Plugins(PluginsArgs),

    #[command(
        alias = "rp",
        name = "remove-plugin",
        about = "Remove a plugin.",
        long_about = "Remove a plugin from the local .prototools config, or global ~/.proto/config.toml config."
    )]
    RemovePlugin(RemovePluginArgs),

    #[command(
        alias = "r",
        name = "run",
        about = "Run a tool after detecting a version from the environment.",
        long_about = "Run a tool after detecting a version from the environment. In order of priority,\na version will be resolved from a provided CLI argument, a PROTO_VERSION environment variable,\na local version file (.prototools), and lastly a global version file (~/.proto/tools/version).\n\nIf no version can be found, the program will exit with an error."
    )]
    Run(RunArgs),

    #[command(name = "setup", about = "Setup proto for your current shell.")]
    Setup(SetupArgs),

    #[command(name = "tools", about = "List all installed tools and their versions.")]
    Tools(ToolsArgs),

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
