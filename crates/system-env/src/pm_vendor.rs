use std::collections::HashMap;

macro_rules! string_vec {
    ($($item:expr),+ $(,)?) => {{
        vec![
            $( String::from($item), )*
        ]
    }};
}

#[derive(Clone, Debug)]
pub struct ListParser {
    regex: regex::Regex,
}

impl ListParser {
    pub fn new(pattern: &str) -> Self {
        let pattern = pattern
            .replace("<package>", "(?<package>[a-zA-Z0-9-_]+)")
            .replace(
                "<version>",
                "(?<version>(\\d+)(\\.\\d+)?(\\.\\d+)?(-[a-z0-9-_+.]+)?)",
            )
            .replace("<distro>", "(?<distro>[a-zA-Z0-9-_,]+)")
            .replace("<tag>", "(?<tag>@[0-9.]+)?")
            .replace("<arch>", "(x86_64|amd64|aarch64|arm64)");

        ListParser {
            regex: regex::Regex::new(&pattern).unwrap(),
        }
    }

    pub fn parse(&self, output: &str) -> HashMap<String, Option<String>> {
        let mut packages = HashMap::default();

        for line in output.lines() {
            let line = line.trim();

            if line.starts_with('#') || line.starts_with("//") {
                continue;
            }

            if let Some(caps) = self.regex.captures(line) {
                let Some(name) = caps.name("package") else {
                    continue;
                };

                packages.insert(
                    name.as_str().to_string(),
                    caps.name("version").map(|cap| cap.as_str().to_string()),
                );
            }
        }

        packages
    }
}

/// The types of commands that are currently supported by package managers.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CommandType {
    /// Installs a system dependency.
    InstallPackage,

    /// Return all installed dependencies.
    ListPackages,

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

    /// Parser for extracting information from installed lists.
    pub list_parser: ListParser,

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
            // https://www.cyberciti.biz/faq/alpine-linux-apk-list-files-in-package/
            (
                CommandType::ListPackages,
                string_vec!["apk", "list", "--installed"],
            ),
            (CommandType::UpdateIndex, string_vec!["apk", "update"]),
        ]),
        list_parser: ListParser::new("^<package>-<version>"),
        prompt_arg: PromptArgument::Interactive("-i".into()),
        prompt_for: vec![CommandType::InstallPackage, CommandType::UpdateIndex],
        version_arg: VersionArgument::Inline("=".into()),
    }
}

// https://manpages.ubuntu.com/manpages/xenial/man8/apt.8.html
pub(crate) fn apt() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (
                CommandType::InstallPackage,
                string_vec!["apt", "install", "--install-recommends", "$"],
            ),
            // https://www.cyberciti.biz/faq/ubuntu-lts-debian-linux-apt-command-examples/#12
            (
                CommandType::ListPackages,
                string_vec!["apt", "list", "--installed"],
            ),
            (CommandType::UpdateIndex, string_vec!["apt", "update"]),
        ]),
        list_parser: ListParser::new("^<package>/<distro> <version>"),
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
            (
                CommandType::ListPackages,
                string_vec!["brew", "list", "--formula", "--versions"],
            ),
            (CommandType::UpdateIndex, string_vec!["brew", "update"]),
        ]),
        list_parser: ListParser::new("^<package><tag> <version>"),
        prompt_arg: PromptArgument::None, // Interactive("-i".into()),
        prompt_for: vec![CommandType::InstallPackage],
        version_arg: VersionArgument::Inline("@".into()),
    }
}

pub(crate) fn choco() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (
                CommandType::InstallPackage,
                string_vec!["choco", "install", "$"],
            ),
            // https://docs.chocolatey.org/en-us/choco/commands/list/
            (CommandType::ListPackages, string_vec!["choco", "list"]),
        ]),
        list_parser: ListParser::new("^<package> <version>"),
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
            // https://tylersguides.com/guides/listing-installed-packages-with-dnf/
            (
                CommandType::ListPackages,
                string_vec!["dnf", "list", "installed"],
            ),
            (CommandType::UpdateIndex, string_vec!["dnf", "check-update"]),
        ]),
        list_parser: ListParser::new("^<package>.<arch>\\s+<version>"),
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
            // https://www.atlantic.net/dedicated-server-hosting/list-installed-packages-with-pacman-on-arch-linux/
            (CommandType::ListPackages, string_vec!["pacman", "-Q"]),
            (CommandType::UpdateIndex, string_vec!["pacman", "-Syy"]),
        ]),
        list_parser: ListParser::new("^<package> <version>"),
        prompt_arg: PromptArgument::Skip("--noconfirm".into()),
        prompt_for: vec![CommandType::InstallPackage],
        version_arg: VersionArgument::Inline(">=".into()),
    }
}

// https://man.freebsd.org/cgi/man.cgi?query=pkg&sektion=8&manpath=freebsd-release-ports
pub(crate) fn pkg() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (
                CommandType::InstallPackage,
                string_vec!["pkg", "install", "$"],
            ),
            // https://www.zenarmor.com/docs/freebsd-tutorials/how-to-install-packages-with-pkg-on-freebsd
            (
                CommandType::ListPackages,
                string_vec!["pkg", "info", "--all"],
            ),
            (CommandType::UpdateIndex, string_vec!["pkg", "update"]),
        ]),
        list_parser: ListParser::new("^<package>-<version>"),
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

// https://pkgin.net/
pub(crate) fn pkgin() -> PackageManagerConfig {
    PackageManagerConfig {
        commands: HashMap::from_iter([
            (
                CommandType::InstallPackage,
                string_vec!["pkgin", "install", "$"],
            ),
            // https://www.unitedbsd.com/d/571-pkgin-statslists-return-seems-false/29
            (CommandType::ListPackages, string_vec!["pkgin", "list"]),
            (CommandType::UpdateIndex, string_vec!["pkgin", "update"]),
        ]),
        list_parser: ListParser::new("^<package>-<version>"),
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
            // https://rcjach.github.io/blog/scoop/#scoop-list
            (CommandType::ListPackages, string_vec!["scoop", "list"]),
            (CommandType::UpdateIndex, string_vec!["scoop", "update"]),
        ]),
        list_parser: ListParser::new("^<package> <version>"),
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
            // https://phoenixnap.com/kb/how-to-list-installed-packages-on-centos
            (
                CommandType::ListPackages,
                string_vec!["yum", "list", "installed"],
            ),
            (CommandType::UpdateIndex, string_vec!["yum", "check-update"]),
        ]),
        list_parser: ListParser::new("^<package>.<arch>\\s+<version>"),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![CommandType::InstallPackage],
        version_arg: VersionArgument::Inline("-".into()),
    }
}
