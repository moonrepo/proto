use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use proto_core::AliasOrVersion;
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
        name = "a",
        name = "alias",
        about = "Add an alias to a tool.",
        long_about = "Add an alias to a tool, that maps to a specific version, or another alias."
    )]
    Alias {
        #[arg(required = true, help = "ID of tool")]
        tool: String,

        #[arg(required = true, help = "Alias name")]
        alias: AliasOrVersion,

        #[arg(required = true, help = "Version (or alias) to associate with")]
        semver: AliasOrVersion,
    },

    #[command(
        name = "bin",
        about = "Display the absolute path to a tools binary.",
        long_about = "Display the absolute path to a tools binary. If no version is provided,\nit will detected from the current environment."
    )]
    Bin {
        #[arg(required = true, help = "ID of tool")]
        tool: String,

        #[arg(help = "Version or alias of tool")]
        semver: Option<AliasOrVersion>,

        #[arg(long, help = "Display shim path when available")]
        shim: bool,
    },

    #[command(
        name = "clean",
        about = "Clean the ~/.proto directory by removing stale and old tools."
    )]
    Clean {
        #[arg(long, help = "Clean tools older than the specified number of days")]
        days: Option<u8>,

        #[arg(long, help = "Avoid and force confirm prompts")]
        yes: bool,
    },

    #[command(
        name = "completions",
        about = "Generate command completions for your current shell."
    )]
    Completions {
        #[arg(long, help = "Shell to generate for")]
        shell: Option<Shell>,
    },

    #[command(
        alias = "i",
        name = "install",
        about = "Download and install a tool.",
        long_about = "Download and install a tool by unpacking the archive to ~/.proto/tools."
    )]
    Install {
        #[arg(required = true, help = "ID of tool")]
        tool: String,

        #[arg(default_value = "latest", help = "Version or alias of tool")]
        semver: Option<AliasOrVersion>,

        #[arg(long, help = "Pin version as the global default")]
        pin: bool,

        // Passthrough args (after --)
        #[arg(last = true, help = "Unique arguments to pass to each tool")]
        passthrough: Vec<String>,
    },

    #[command(
        alias = "ig",
        name = "install-global",
        about = "Install a global dependency for the specified tool.",
        long_about = "Install a global dependency for the specified tool. Depending on the tool, the dependency will either be installed to ~/.proto/tools/<tool>/globals or ~/<tool>."
    )]
    InstallGlobal {
        #[arg(required = true, help = "ID of tool")]
        tool: String,

        #[arg(required = true, help = "Dependencies and optional version to install")]
        dependencies: Vec<String>,
    },

    #[command(
        name = "global",
        about = "Set the global default version of a tool.",
        long_about = "Set the global default version of a tool. This will pin the version in the ~/.proto/tools installation directory."
    )]
    Global {
        #[arg(required = true, help = "ID of tool")]
        tool: String,

        #[arg(required = true, help = "Version or alias of tool")]
        semver: AliasOrVersion,
    },

    #[command(
        name = "list",
        alias = "ls",
        about = "List installed versions.",
        long_about = "List installed versions by scanning the ~/.proto/tools directory for possible versions."
    )]
    List {
        #[arg(required = true, help = "ID of tool")]
        tool: String,
    },

    #[command(
        name = "list-global",
        alias = "lsg",
        about = "List installed globals.",
        long_about = "List installed globals by scanning the global bins installation directory. Will return the canonical source path."
    )]
    ListGlobal {
        #[arg(required = true, help = "ID of tool")]
        tool: String,
    },

    #[command(
        name = "list-remote",
        alias = "lsr",
        about = "List available versions.",
        long_about = "List available versions by resolving versions from the tool's remote release manifest."
    )]
    ListRemote {
        #[arg(required = true, help = "ID of tool")]
        tool: String,
    },

    #[command(
        name = "local",
        about = "Set the local version of a tool.",
        long_about = "Set the local version of a tool. This will create a .prototools file (if it does not exist)\nin the current working directory with the appropriate tool and version."
    )]
    Local {
        #[arg(required = true, help = "ID of tool")]
        tool: String,

        #[arg(required = true, help = "Version or alias of tool")]
        semver: AliasOrVersion,
    },

    #[command(name = "plugins", about = "List all active and configured plugins.")]
    Plugins {
        #[arg(long, help = "Print the list in JSON format")]
        json: bool,
    },

    #[command(
        alias = "r",
        name = "run",
        about = "Run a tool after detecting a version from the environment.",
        long_about = "Run a tool after detecting a version from the environment. In order of priority,\na version will be resolved from a provided CLI argument, a PROTO_VERSION environment variable,\na local version file (.prototools), and lastly a global version file (~/.proto/tools/version).\n\nIf no version can be found, the program will exit with an error."
    )]
    Run {
        #[arg(required = true, help = "ID of tool")]
        tool: String,

        #[arg(help = "Version or alias of tool")]
        semver: Option<AliasOrVersion>,

        #[arg(long, help = "Path to an alternate binary to run")]
        bin: Option<String>,

        // Passthrough args (after --)
        #[arg(
            last = true,
            help = "Arguments to pass through to the underlying command"
        )]
        passthrough: Vec<String>,
    },

    #[command(name = "setup", about = "Setup proto for your current shell.")]
    Setup {
        #[arg(long, help = "Shell to setup for")]
        shell: Option<Shell>,

        #[arg(long, help = "Return the profile path if setup")]
        profile: bool,
    },

    #[command(name = "ua", name = "unalias", about = "Remove an alias from a tool.")]
    Unalias {
        #[arg(required = true, help = "ID of tool")]
        tool: String,

        #[arg(required = true, help = "Alias name")]
        alias: String,
    },

    #[command(
        name = "ui",
        name = "uninstall",
        about = "Uninstall a tool.",
        long_about = "Uninstall a tool and remove the installation from ~/.proto/tools."
    )]
    Uninstall {
        #[arg(required = true, help = "ID of tool")]
        tool: String,

        #[arg(required = true, help = "Version or alias of tool")]
        semver: AliasOrVersion,
    },

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
