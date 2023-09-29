use std::collections::HashMap;

macro_rules! string_vec {
    ($($item:expr),+ $(,)?) => {{
        vec![
            $( String::from($item), )*
        ]
    }};
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Command {
    InstallPackage,
    UpdateIndex,
}

#[derive(Clone, Debug)]
pub enum PromptArgument {
    None,
    // -i
    Interactive(String),
    // -y
    Skip(String),
}

#[derive(Clone, Debug)]
pub enum VersionArgument {
    None,
    // pkg=1.2.3
    Inline(String),
    // pkg --version 1.2.3
    Separate(String),
}

#[derive(Clone, Debug)]
pub struct PackageVendorConfig {
    pub commands: HashMap<Command, Vec<String>>,
    pub prompt_arg: PromptArgument,
    pub prompt_for: Vec<Command>,
    pub version_arg: VersionArgument,
}

pub fn apk() -> PackageVendorConfig {
    PackageVendorConfig {
        commands: HashMap::from_iter([
            (Command::InstallPackage, string_vec!["apk", "add", "$"]),
            (Command::UpdateIndex, string_vec!["apk", "update"]),
        ]),
        prompt_arg: PromptArgument::Interactive("-i".into()),
        prompt_for: vec![Command::InstallPackage, Command::UpdateIndex],
        version_arg: VersionArgument::Inline("=".into()),
    }
}

pub fn apt() -> PackageVendorConfig {
    PackageVendorConfig {
        commands: HashMap::from_iter([
            (
                Command::InstallPackage,
                string_vec!["apt", "install", "--install-recommends", "$"],
            ),
            (Command::UpdateIndex, string_vec!["apt", "update"]),
        ]),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![Command::InstallPackage, Command::UpdateIndex],
        version_arg: VersionArgument::Inline("=".into()),
    }
}

pub fn brew() -> PackageVendorConfig {
    PackageVendorConfig {
        commands: HashMap::from_iter([
            (Command::InstallPackage, string_vec!["brew", "install", "$"]),
            (Command::UpdateIndex, string_vec!["brew", "update"]),
        ]),
        prompt_arg: PromptArgument::Interactive("-i".into()),
        prompt_for: vec![Command::InstallPackage],
        version_arg: VersionArgument::Inline("@".into()),
    }
}

pub fn choco() -> PackageVendorConfig {
    PackageVendorConfig {
        commands: HashMap::from_iter([(
            Command::InstallPackage,
            string_vec!["choco", "install", "$"],
        )]),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![Command::InstallPackage],
        version_arg: VersionArgument::Separate("--version".into()),
    }
}

pub fn dnf() -> PackageVendorConfig {
    PackageVendorConfig {
        commands: HashMap::from_iter([
            (Command::InstallPackage, string_vec!["dnf", "install", "$"]),
            (Command::UpdateIndex, string_vec!["dnf", "check-update"]),
        ]),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![Command::InstallPackage, Command::UpdateIndex],
        version_arg: VersionArgument::Inline("-".into()),
    }
}

pub fn pacman() -> PackageVendorConfig {
    PackageVendorConfig {
        commands: HashMap::from_iter([
            (Command::InstallPackage, string_vec!["pacman", "-S", "$"]),
            (Command::UpdateIndex, string_vec!["pacman", "-Syy"]),
        ]),
        prompt_arg: PromptArgument::Skip("--noconfirm".into()),
        prompt_for: vec![Command::InstallPackage],
        version_arg: VersionArgument::Inline(">=".into()),
    }
}

pub fn pkg() -> PackageVendorConfig {
    PackageVendorConfig {
        commands: HashMap::from_iter([
            (Command::InstallPackage, string_vec!["pkg", "install", "$"]),
            (Command::UpdateIndex, string_vec!["pkg", "update"]),
        ]),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![Command::InstallPackage],
        version_arg: VersionArgument::None,
    }
}

pub fn pkg_alt() -> PackageVendorConfig {
    PackageVendorConfig {
        commands: HashMap::from_iter([(Command::InstallPackage, string_vec!["pkg_add", "$"])]),
        prompt_arg: PromptArgument::Skip("-I".into()),
        prompt_for: vec![Command::InstallPackage],
        version_arg: VersionArgument::None,
    }
}

pub fn pkgin() -> PackageVendorConfig {
    PackageVendorConfig {
        commands: HashMap::from_iter([
            (
                Command::InstallPackage,
                string_vec!["pkgin", "install", "$"],
            ),
            (Command::UpdateIndex, string_vec!["pkgin", "update"]),
        ]),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![Command::InstallPackage, Command::UpdateIndex],
        version_arg: VersionArgument::Inline("-".into()),
    }
}

pub fn scoop() -> PackageVendorConfig {
    PackageVendorConfig {
        commands: HashMap::from_iter([
            (
                Command::InstallPackage,
                string_vec!["scoop", "install", "$"],
            ),
            (Command::UpdateIndex, string_vec!["scoop", "update"]),
        ]),
        prompt_arg: PromptArgument::None,
        prompt_for: vec![],
        version_arg: VersionArgument::Inline("@".into()),
    }
}

pub fn yum() -> PackageVendorConfig {
    PackageVendorConfig {
        commands: HashMap::from_iter([
            (Command::InstallPackage, string_vec!["yum", "install", "$"]),
            (Command::UpdateIndex, string_vec!["yum", "check-update"]),
        ]),
        prompt_arg: PromptArgument::Skip("-y".into()),
        prompt_for: vec![Command::InstallPackage],
        version_arg: VersionArgument::Inline("-".into()),
    }
}
