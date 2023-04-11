use crate::tools::ToolType;
use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Debug, Parser)]
#[command(
    name = "proto",
    version,
    about,
    long_about = None,
    disable_colored_help = true,
    disable_help_subcommand = true,
    propagate_version = true,
    next_line_help = false
)]
pub struct App {
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
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(required = true, help = "Alias name")]
        alias: String,

        #[arg(required = true, help = "Version (or alias) to associate with")]
        semver: String,
    },

    #[command(
        name = "bin",
        about = "Display the absolute path to a tools binary.",
        long_about = "Display the absolute path to a tools binary. If no version is provided,\nit will detected from the current environment."
    )]
    Bin {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(help = "Version of tool")]
        semver: Option<String>,

        #[arg(long, help = "Display shim path when available")]
        shim: bool,
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
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(default_value = "latest", help = "Version of tool")]
        semver: Option<String>,

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
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(required = true, help = "Dependencies and optional version to install")]
        dependencies: Vec<String>,
    },

    #[command(
        name = "global",
        about = "Set the global default version of a tool.",
        long_about = "Set the global default version of a tool. This will pin the version in the ~/.proto/tools installation directory."
    )]
    Global {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(required = true, help = "Version of tool")]
        semver: String,
    },

    #[command(
        name = "list",
        alias = "ls",
        about = "List installed versions.",
        long_about = "List installed versions by scanning the ~/.proto/tools directory for possible versions."
    )]
    List {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,
    },

    #[command(
        name = "list-global",
        alias = "lsg",
        about = "List installed globals.",
        long_about = "List installed globals by scanning the global bins installation directory. Will return the canonical source path."
    )]
    ListGlobal {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,
    },

    #[command(
        name = "list-remote",
        alias = "lsr",
        about = "List available versions.",
        long_about = "List available versions by resolving versions from the tool's remote release manifest."
    )]
    ListRemote {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,
    },

    #[command(
        name = "local",
        about = "Set the local version of a tool.",
        long_about = "Set the local version of a tool. This will create a .prototools file (if it does not exist)\nin the current working directory with the appropriate tool and version."
    )]
    Local {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(required = true, help = "Version of tool")]
        semver: String,
    },

    #[command(
        alias = "r",
        name = "run",
        about = "Run a tool after detecting a version from the environment.",
        long_about = "Run a tool after detecting a version from the environment. In order of priority,\na version will be resolved from a provided CLI argument, a PROTO_VERSION environment variable,\na local version file (.prototools), and lastly a global version file (~/.proto/tools/version).\n\nIf no version can be found, the program will exit with an error."
    )]
    Run {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(help = "Version of tool")]
        semver: Option<String>,

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
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

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
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(required = true, help = "Version of tool")]
        semver: String,
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
