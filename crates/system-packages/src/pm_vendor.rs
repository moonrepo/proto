pub enum VersionArgument {
    None,
    // pkg=1.2.3
    Inline(String),
    // pkg --version 1.2.3
    Separate(String),
}

pub struct VendorConfig {
    pub install_command: Vec<String>,
    pub update_index_command: Option<Vec<String>>,
    pub version_arg: VersionArgument,
}

pub fn apk() -> VendorConfig {
    VendorConfig {
        install_command: vec!["apk".into(), "add".into(), "$".into()],
        update_index_command: Some(vec!["apk".into(), "update".into()]),
        version_arg: VersionArgument::Inline("=".into()),
    }
}

pub fn apt() -> VendorConfig {
    VendorConfig {
        install_command: vec!["apt".into(), "install".into(), "-y".into(), "$".into()],
        update_index_command: Some(vec!["apt".into(), "update".into()]),
        version_arg: VersionArgument::Inline("=".into()),
    }
}

pub fn brew() -> VendorConfig {
    VendorConfig {
        install_command: vec!["brew".into(), "install".into(), "$".into()],
        update_index_command: Some(vec!["brew".into(), "update".into()]),
        version_arg: VersionArgument::Inline("@".into()),
    }
}

pub fn choco() -> VendorConfig {
    VendorConfig {
        install_command: vec!["choco".into(), "install".into(), "-y".into(), "$".into()],
        update_index_command: None,
        version_arg: VersionArgument::Separate("--version".into()),
    }
}

pub fn dnf() -> VendorConfig {
    VendorConfig {
        install_command: vec!["dnf".into(), "install".into(), "-y".into(), "$".into()],
        update_index_command: Some(vec!["dnf".into(), "check-update".into()]),
        version_arg: VersionArgument::Inline("-".into()),
    }
}

pub fn pacman() -> VendorConfig {
    VendorConfig {
        install_command: vec![
            "pacman".into(),
            "-S".into(),
            "--noconfirm".into(),
            "$".into(),
        ],
        update_index_command: Some(vec!["pacman".into(), "-Syy".into()]),
        version_arg: VersionArgument::Inline(">=".into()),
    }
}

pub fn pkg() -> VendorConfig {
    VendorConfig {
        install_command: vec!["pkg".into(), "install".into(), "-y".into(), "$".into()],
        update_index_command: Some(vec!["pkg".into(), "update".into()]),
        version_arg: VersionArgument::None,
    }
}

pub fn pkg_alt() -> VendorConfig {
    VendorConfig {
        install_command: vec!["pkg_add".into(), "$".into()],
        update_index_command: None,
        version_arg: VersionArgument::None,
    }
}

pub fn pkgin() -> VendorConfig {
    VendorConfig {
        install_command: vec!["pkgin".into(), "install".into(), "-y".into(), "$".into()],
        update_index_command: Some(vec!["pkgin".into(), "update".into()]),
        version_arg: VersionArgument::Inline("-".into()),
    }
}

pub fn scoop() -> VendorConfig {
    VendorConfig {
        install_command: vec!["scoop".into(), "install".into(), "$".into()],
        update_index_command: Some(vec!["scoop".into(), "update".into()]),
        version_arg: VersionArgument::Inline("@".into()),
    }
}

pub fn yum() -> VendorConfig {
    VendorConfig {
        install_command: vec!["yum".into(), "install".into(), "-y".into(), "$".into()],
        update_index_command: Some(vec!["yum".into(), "check-update".into()]),
        version_arg: VersionArgument::Inline("-".into()),
    }
}
