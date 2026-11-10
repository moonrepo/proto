use system_env::{SystemArch, SystemDetails, SystemDistro, SystemOS, SystemPlatform};

fn distro(id: &str, version_id: Option<&str>) -> Option<SystemDistro> {
    Some(SystemDistro {
        id: id.into(),
        version_id: version_id.map(|version| version.into()),
    })
}

mod os_release {
    use super::*;

    #[test]
    fn parses_ubuntu() {
        let content = r#"PRETTY_NAME="Ubuntu 24.04.1 LTS"
NAME="Ubuntu"
VERSION_ID="24.04"
VERSION="24.04.1 LTS (Noble Numbat)"
VERSION_CODENAME=noble
ID=ubuntu
ID_LIKE=debian
HOME_URL="https://www.ubuntu.com/"
UBUNTU_CODENAME=noble
"#;

        assert_eq!(
            SystemDistro::parse_os_release(content),
            distro("ubuntu", Some("24.04"))
        );
    }

    #[test]
    fn parses_debian() {
        let content = r#"PRETTY_NAME="Debian GNU/Linux 12 (bookworm)"
NAME="Debian GNU/Linux"
VERSION_ID="12"
VERSION="12 (bookworm)"
VERSION_CODENAME=bookworm
ID=debian
HOME_URL="https://www.debian.org/"
"#;

        assert_eq!(
            SystemDistro::parse_os_release(content),
            distro("debian", Some("12"))
        );
    }

    #[test]
    fn parses_fedora() {
        let content = r#"NAME="Fedora Linux"
VERSION="40 (Container Image)"
ID=fedora
VERSION_ID=40
VERSION_CODENAME=""
PLATFORM_ID="platform:f40"
PRETTY_NAME="Fedora Linux 40 (Container Image)"
"#;

        assert_eq!(
            SystemDistro::parse_os_release(content),
            distro("fedora", Some("40"))
        );
    }

    #[test]
    fn parses_alpine() {
        let content = r#"NAME="Alpine Linux"
ID=alpine
VERSION_ID=3.20.3
PRETTY_NAME="Alpine Linux v3.20"
HOME_URL="https://alpinelinux.org/"
"#;

        assert_eq!(
            SystemDistro::parse_os_release(content),
            distro("alpine", Some("3.20.3"))
        );
    }

    #[test]
    fn parses_without_version() {
        let content = r#"NAME="Arch Linux"
PRETTY_NAME="Arch Linux"
ID=arch
BUILD_ID=rolling
"#;

        assert_eq!(
            SystemDistro::parse_os_release(content),
            distro("arch", None)
        );
    }

    #[test]
    fn supports_comments_single_quotes_and_escapes() {
        let content = r#"# A comment
ID='Custom'
VERSION_ID="1.0 \"beta\""
"#;

        assert_eq!(
            SystemDistro::parse_os_release(content),
            distro("custom", Some(r#"1.0 "beta""#))
        );
    }

    #[test]
    fn returns_none_without_id() {
        assert_eq!(SystemDistro::parse_os_release(""), None);
        assert_eq!(
            SystemDistro::parse_os_release("NAME=\"Linux\"\nVERSION_ID=1\n"),
            None
        );
        assert_eq!(SystemDistro::parse_os_release("ID=\"\"\n"), None);
    }

    #[test]
    fn loads_from_file() {
        let dir = std::env::temp_dir().join(format!("system-env-details-{}", std::process::id()));
        let file = dir.join("os-release");

        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, "ID=debian\nVERSION_ID=\"12\"\n").unwrap();

        let result = SystemDistro::from_file(&file);

        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(result, distro("debian", Some("12")));
    }

    #[test]
    fn returns_none_for_missing_file() {
        assert_eq!(
            SystemDistro::from_file(std::env::temp_dir().join("system-env-missing-os-release")),
            None
        );
    }
}

mod arm_version {
    use super::*;

    #[test]
    fn parses_32bit_arm() {
        assert_eq!(SystemDetails::parse_arm_version("armv6l"), Some(6));
        assert_eq!(SystemDetails::parse_arm_version("armv7l\n"), Some(7));
        assert_eq!(SystemDetails::parse_arm_version("armv7"), Some(7));
        assert_eq!(SystemDetails::parse_arm_version("armv8l"), Some(8));
    }

    #[test]
    fn returns_none_for_other_machines() {
        assert_eq!(SystemDetails::parse_arm_version("aarch64"), None);
        assert_eq!(SystemDetails::parse_arm_version("arm64"), None);
        assert_eq!(SystemDetails::parse_arm_version("x86_64"), None);
        assert_eq!(SystemDetails::parse_arm_version("arm"), None);
        assert_eq!(SystemDetails::parse_arm_version(""), None);
    }
}

mod x64_level {
    use super::*;

    #[test]
    fn requires_every_lower_level() {
        assert_eq!(SystemDetails::get_x64_level(false, false, false), 1);
        assert_eq!(SystemDetails::get_x64_level(true, false, false), 2);
        assert_eq!(SystemDetails::get_x64_level(true, true, false), 3);
        assert_eq!(SystemDetails::get_x64_level(true, true, true), 4);
        assert_eq!(SystemDetails::get_x64_level(false, true, true), 1);
        assert_eq!(SystemDetails::get_x64_level(true, false, true), 2);
    }

    #[test]
    fn only_detected_on_x64() {
        let details = SystemDetails::from_env();

        if cfg!(target_arch = "x86_64") {
            assert!(
                details
                    .x64_level
                    .is_some_and(|level| (1..=4).contains(&level))
            );
        } else {
            assert_eq!(details.x64_level, None);
        }
    }
}

mod from_platform {
    use super::*;

    #[test]
    fn matches_from_env_for_this_machine() {
        assert_eq!(
            SystemDetails::from_env(),
            SystemDetails::from_platform(&SystemPlatform::new(
                SystemOS::from_env(),
                SystemArch::from_env()
            ))
        );
    }

    #[test]
    fn only_detects_details_that_apply_to_the_platform() {
        // No distribution outside of Linux, no ARM version outside of 32-bit
        // ARM, and no x86-64 level outside of x64 (x86 is 32-bit)
        for platform in [
            SystemPlatform::new(SystemOS::MacOS, SystemArch::Arm64),
            SystemPlatform::new(SystemOS::Windows, SystemArch::X86),
        ] {
            assert_eq!(
                SystemDetails::from_platform(&platform),
                SystemDetails::default()
            );
        }
    }
}

mod serde {
    use super::*;

    #[test]
    fn omits_missing_details() {
        assert_eq!(
            serde_json::to_string(&SystemDetails::default()).unwrap(),
            "{}"
        );
        assert_eq!(
            serde_json::from_str::<SystemDetails>("{}").unwrap(),
            SystemDetails::default()
        );
    }
}
