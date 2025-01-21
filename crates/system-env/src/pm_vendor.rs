use std::collections::HashMap;

macro_rules! string_vec {
    ($($item:expr),+ $(,)?) => {{
        vec![
            $( String::from($item), )*
        ]
    }};
}

/// The types of commands that are currently supported by package managers.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CommandType {
    /// Installs a system dependency.
    InstallPackage,

    /// Updates the registry index.
    UpdateIndex,
}

/// The CLI argument format for enabling interactive mode.
#[derive(Clone, Debug)]
pub enum PromptArgument {
    /// Does not support interactive.
    None,
    /// Enables interactive mode: `-i`
    Interactive(String),
    /// Disables interactive mode: `-y`
    Skip(String),
}

/// The CLI argument format for including the package version to install.
#[derive(Clone, Debug)]
pub enum VersionArgument {
    /// Does not support versions.
    None,
    /// In the same argument with the package name: `pkg=1.2.3`
    Inline(String),
    /// As a separate argument: `pkg --version 1.2.3`
    Separate(String),
}

/// Configuration for a specific package manager vendor.
/// The fields define commands and arguments for common operations.
#[derive(Clone, Debug)]
pub struct PackageManagerConfig {
    /// Mapping of command types to CLI arguments.
    pub commands: HashMap<CommandType, Vec<String>>,

    /// How interactive/prompt arguments are handled.
    pub prompt_arg: PromptArgument,

    /// List of commands that support prompts.
    pub prompt_for: Vec<CommandType>,

    /// How version arguments are handled.
    pub version_arg: VersionArgument,
}

pub(crate) fn apk() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (CommandType::InstallPackage, string_vec!["apk", "add", "$"]),
            (CommandType::UpdateIndex, string_vec!["apk", "update"]),
        ]),
        prompt_arg: PromptArgument::Interactive("-i".into()),
        prompt_for: vec![CommandType::InstallPackage, CommandType::UpdateIndex],
        version_arg: VersionArgument::Inline("=".into()),
    }
}

pub(crate) fn apt() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (
                CommandType::InstallPackage,
                string_vec!["apt", "install", "--install-recommends", "$"],
            ),
            (CommandType::UpdateIndex, string_vec!["apt", "update"]),
        ]),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![CommandType::InstallPackage, CommandType::UpdateIndex],
        version_arg: VersionArgument::Inline("=".into()),
    }
}

pub(crate) fn brew() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (
                CommandType::InstallPackage,
                string_vec!["brew", "install", "$"],
            ),
            (CommandType::UpdateIndex, string_vec!["brew", "update"]),
        ]),
        prompt_arg: PromptArgument::None, // Interactive("-i".into()),
        prompt_for: vec![CommandType::InstallPackage],
        version_arg: VersionArgument::Inline("@".into()),
    }
}

pub(crate) fn choco() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([(
            CommandType::InstallPackage,
            string_vec!["choco", "install", "$"],
        )]),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![CommandType::InstallPackage],
        version_arg: VersionArgument::Separate("--version".into()),
    }
}

pub(crate) fn dnf() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (
                CommandType::InstallPackage,
                string_vec!["dnf", "install", "$"],
            ),
            (CommandType::UpdateIndex, string_vec!["dnf", "check-update"]),
        ]),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![CommandType::InstallPackage, CommandType::UpdateIndex],
        version_arg: VersionArgument::Inline("-".into()),
    }
}

pub(crate) fn pacman() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (
                CommandType::InstallPackage,
                string_vec!["pacman", "-S", "$"],
            ),
            (CommandType::UpdateIndex, string_vec!["pacman", "-Syy"]),
        ]),
        prompt_arg: PromptArgument::Skip("--noconfirm".into()),
        prompt_for: vec![CommandType::InstallPackage],
        version_arg: VersionArgument::Inline(">=".into()),
    }
}

pub(crate) fn pkg() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (
                CommandType::InstallPackage,
                string_vec!["pkg", "install", "$"],
            ),
            (CommandType::UpdateIndex, string_vec!["pkg", "update"]),
        ]),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![CommandType::InstallPackage],
        version_arg: VersionArgument::None,
    }
}

// pub(crate) fn pkg_alt() -> PackageVendorConfig {
//     PackageVendorConfig {
//         commands: HashMap::from_iter([(Command::InstallPackage, string_vec!["pkg_add", "$"])]),
//         prompt_arg: PromptArgument::Skip("-I".into()),
//         prompt_for: vec![Command::InstallPackage],
//         version_arg: VersionArgument::None,
//     }
// }

pub(crate) fn pkgin() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (
                CommandType::InstallPackage,
                string_vec!["pkgin", "install", "$"],
            ),
            (CommandType::UpdateIndex, string_vec!["pkgin", "update"]),
        ]),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![CommandType::InstallPackage, CommandType::UpdateIndex],
        version_arg: VersionArgument::Inline("-".into()),
    }
}

pub(crate) fn scoop() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (
                CommandType::InstallPackage,
                string_vec!["scoop", "install", "$"],
            ),
            (CommandType::UpdateIndex, string_vec!["scoop", "update"]),
        ]),
        prompt_arg: PromptArgument::None,
        prompt_for: vec![],
        version_arg: VersionArgument::Inline("@".into()),
    }
}

pub(crate) fn yum() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (
                CommandType::InstallPackage,
                string_vec!["yum", "install", "$"],
            ),
            (CommandType::UpdateIndex, string_vec!["yum", "check-update"]),
        ]),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![CommandType::InstallPackage],
        version_arg: VersionArgument::Inline("-".into()),
    }
}
