use system_env::*;

mod system_platform {
    use super::*;

    fn platform(arch: SystemArch, os: SystemOS, libc: SystemLibc) -> SystemPlatform {
        SystemPlatform { os, arch, libc }
    }

    mod parse {
        use super::*;

        #[test]
        fn arch_and_os() {
            assert_eq!(
                SystemPlatform::parse("arm64-macos").unwrap(),
                platform(SystemArch::Arm64, SystemOS::MacOS, SystemLibc::Unknown)
            );
            assert_eq!(
                SystemPlatform::parse("x64-windows").unwrap(),
                platform(SystemArch::X64, SystemOS::Windows, SystemLibc::Unknown)
            );
        }

        #[test]
        fn defaults_to_gnu_on_linux() {
            assert_eq!(
                SystemPlatform::parse("x64-linux").unwrap(),
                platform(SystemArch::X64, SystemOS::Linux, SystemLibc::Gnu)
            );
        }

        #[test]
        fn arch_os_and_libc() {
            assert_eq!(
                SystemPlatform::parse("arm64-linux-musl").unwrap(),
                platform(SystemArch::Arm64, SystemOS::Linux, SystemLibc::Musl)
            );
            assert_eq!(
                SystemPlatform::parse("x64-linux-gnu").unwrap(),
                platform(SystemArch::X64, SystemOS::Linux, SystemLibc::Gnu)
            );
            assert_eq!(
                SystemPlatform::parse("x64-linux-unknown").unwrap(),
                platform(SystemArch::X64, SystemOS::Linux, SystemLibc::Unknown)
            );
        }

        #[test]
        fn supports_aliases() {
            assert_eq!(
                SystemPlatform::parse("aarch64-mac").unwrap(),
                SystemPlatform::new(SystemOS::MacOS, SystemArch::Arm64)
            );
            assert_eq!(
                SystemPlatform::parse("x86_64-linux-glibc").unwrap(),
                SystemPlatform::new(SystemOS::Linux, SystemArch::X64)
            );
            assert_eq!(
                SystemPlatform::parse("loongarch64-linux").unwrap(),
                SystemPlatform::new(SystemOS::Linux, SystemArch::LongArm64)
            );
        }

        #[test]
        fn is_case_insensitive_and_trims() {
            assert_eq!(
                SystemPlatform::parse("  X64-Linux-MUSL ").unwrap(),
                SystemPlatform::new(SystemOS::Linux, SystemArch::X64).with_libc(SystemLibc::Musl)
            );
        }

        #[test]
        fn errors_for_invalid_format() {
            for value in [
                "",
                "x64",
                "x64-",
                "-linux",
                "x64--musl",
                "x64-linux-",
                "x64-linux-musl-extra",
                "x86_64-unknown-linux-gnu-extra",
            ] {
                assert!(
                    matches!(SystemPlatform::parse(value), Err(Error::InvalidPlatform(_))),
                    "{value}"
                );
            }
        }

        #[test]
        fn errors_for_unknown_parts() {
            for (value, expected_kind, expected_value) in [
                ("z80-linux", "architecture", "z80"),
                ("x64-beos", "operating system", "beos"),
                ("x86_64-unknown-beos", "operating system", "beos"),
                ("x86_64-pc-beos-gnu", "operating system", "beos"),
                ("x64-linux-bionic", "libc", "bionic"),
                ("x86_64-unknown-linux-uclibc", "libc", "uclibc"),
            ] {
                let result = SystemPlatform::parse(value);

                assert!(
                    matches!(
                        &result,
                        Err(Error::UnknownPlatformPart { kind, value, .. })
                            if kind == expected_kind && value == expected_value
                    ),
                    "{value}: {result:?}"
                );
            }
        }

        // The OS is parsed first, so the old `<os>-<arch>` format is not supported
        #[test]
        fn errors_for_os_first() {
            assert!(matches!(
                SystemPlatform::parse("linux-x64"),
                Err(Error::UnknownPlatformPart { kind, .. }) if kind == "architecture"
            ));
        }
    }

    mod parse_rust_triple {
        use super::*;

        #[test]
        fn tier_1_and_2_hosts() {
            for (triple, expected) in [
                (
                    "aarch64-apple-darwin",
                    platform(SystemArch::Arm64, SystemOS::MacOS, SystemLibc::Unknown),
                ),
                (
                    "x86_64-apple-darwin",
                    platform(SystemArch::X64, SystemOS::MacOS, SystemLibc::Unknown),
                ),
                (
                    "x86_64-unknown-linux-gnu",
                    platform(SystemArch::X64, SystemOS::Linux, SystemLibc::Gnu),
                ),
                (
                    "aarch64-unknown-linux-gnu",
                    platform(SystemArch::Arm64, SystemOS::Linux, SystemLibc::Gnu),
                ),
                (
                    "x86_64-unknown-linux-musl",
                    platform(SystemArch::X64, SystemOS::Linux, SystemLibc::Musl),
                ),
                (
                    "aarch64-unknown-linux-musl",
                    platform(SystemArch::Arm64, SystemOS::Linux, SystemLibc::Musl),
                ),
                (
                    "x86_64-pc-windows-msvc",
                    platform(SystemArch::X64, SystemOS::Windows, SystemLibc::Unknown),
                ),
                (
                    "aarch64-pc-windows-msvc",
                    platform(SystemArch::Arm64, SystemOS::Windows, SystemLibc::Unknown),
                ),
                (
                    "x86_64-pc-windows-gnu",
                    platform(SystemArch::X64, SystemOS::Windows, SystemLibc::Gnu),
                ),
                (
                    "i686-unknown-linux-gnu",
                    platform(SystemArch::X86, SystemOS::Linux, SystemLibc::Gnu),
                ),
                (
                    "i686-pc-windows-msvc",
                    platform(SystemArch::X86, SystemOS::Windows, SystemLibc::Unknown),
                ),
                (
                    "x86_64-unknown-freebsd",
                    platform(SystemArch::X64, SystemOS::FreeBSD, SystemLibc::Unknown),
                ),
                (
                    "x86_64-unknown-netbsd",
                    platform(SystemArch::X64, SystemOS::NetBSD, SystemLibc::Unknown),
                ),
                (
                    "loongarch64-unknown-linux-gnu",
                    platform(SystemArch::LongArm64, SystemOS::Linux, SystemLibc::Gnu),
                ),
                (
                    "powerpc64le-unknown-linux-gnu",
                    platform(SystemArch::Powerpc64, SystemOS::Linux, SystemLibc::Gnu),
                ),
                (
                    "riscv64gc-unknown-linux-gnu",
                    platform(SystemArch::Riscv64, SystemOS::Linux, SystemLibc::Gnu),
                ),
                (
                    "s390x-unknown-linux-gnu",
                    platform(SystemArch::S390x, SystemOS::Linux, SystemLibc::Gnu),
                ),
                (
                    "sparcv9-sun-solaris",
                    platform(SystemArch::Sparc64, SystemOS::Solaris, SystemLibc::Unknown),
                ),
            ] {
                assert_eq!(SystemPlatform::parse(triple).unwrap(), expected, "{triple}");
            }
        }

        #[test]
        fn arm_variants() {
            for (triple, libc) in [
                ("arm-unknown-linux-gnueabi", SystemLibc::Gnu),
                ("arm-unknown-linux-musleabihf", SystemLibc::Musl),
                ("armv7-unknown-linux-gnueabihf", SystemLibc::Gnu),
                ("armv5te-unknown-linux-musleabi", SystemLibc::Musl),
                ("thumbv7neon-unknown-linux-gnueabihf", SystemLibc::Gnu),
            ] {
                assert_eq!(
                    SystemPlatform::parse(triple).unwrap(),
                    platform(SystemArch::Arm, SystemOS::Linux, libc),
                    "{triple}"
                );
            }
        }

        #[test]
        fn mips_variants() {
            assert_eq!(
                SystemPlatform::parse("mipsel-unknown-linux-gnu").unwrap(),
                platform(SystemArch::Mips, SystemOS::Linux, SystemLibc::Gnu)
            );
            assert_eq!(
                SystemPlatform::parse("mips64el-unknown-linux-gnuabi64").unwrap(),
                platform(SystemArch::Mips64, SystemOS::Linux, SystemLibc::Gnu)
            );
        }

        #[test]
        fn android_without_vendor() {
            for (triple, arch) in [
                ("aarch64-linux-android", SystemArch::Arm64),
                ("x86_64-linux-android", SystemArch::X64),
                ("armv7-linux-androideabi", SystemArch::Arm),
            ] {
                assert_eq!(
                    SystemPlatform::parse(triple).unwrap(),
                    platform(arch, SystemOS::Android, SystemLibc::Unknown),
                    "{triple}"
                );
            }
        }

        #[test]
        fn apple_mobile() {
            assert_eq!(
                SystemPlatform::parse("aarch64-apple-ios").unwrap(),
                platform(SystemArch::Arm64, SystemOS::IOS, SystemLibc::Unknown)
            );
            assert_eq!(
                SystemPlatform::parse("aarch64-apple-ios-sim").unwrap(),
                platform(SystemArch::Arm64, SystemOS::IOS, SystemLibc::Unknown)
            );
            assert_eq!(
                SystemPlatform::parse("arm64e-apple-darwin").unwrap(),
                platform(SystemArch::Arm64, SystemOS::MacOS, SystemLibc::Unknown)
            );
        }

        #[test]
        fn errors_for_unsupported_targets() {
            for triple in [
                "wasm32-wasip1",
                "wasm32-unknown-unknown",
                "thumbv7em-none-eabihf",
                "x86_64-unknown-illumos",
                "nvptx64-nvidia-cuda",
            ] {
                assert!(
                    matches!(
                        SystemPlatform::parse(triple),
                        Err(Error::UnknownPlatformPart { .. })
                    ),
                    "{triple}"
                );
            }
        }
    }

    mod display {
        use super::*;

        #[test]
        fn omits_unknown_libc() {
            assert_eq!(
                SystemPlatform::new(SystemOS::MacOS, SystemArch::Arm64).to_string(),
                "arm64-macos"
            );
        }

        #[test]
        fn includes_known_libc() {
            assert_eq!(
                SystemPlatform::new(SystemOS::Linux, SystemArch::X64).to_string(),
                "x64-linux-gnu"
            );
            assert_eq!(
                SystemPlatform::new(SystemOS::Linux, SystemArch::Arm64)
                    .with_libc(SystemLibc::Musl)
                    .to_string(),
                "arm64-linux-musl"
            );
        }

        #[test]
        fn round_trips() {
            for value in [
                "arm64-macos",
                "x64-macos",
                "arm64-windows",
                "x64-windows-gnu",
                "x64-linux-gnu",
                "arm64-linux-musl",
                "arm64-android",
                "x64-freebsd",
                "x64-freebsd-gnu",
            ] {
                assert_eq!(SystemPlatform::parse(value).unwrap().to_string(), value);
            }
        }

        #[test]
        fn normalizes_rust_triples() {
            for (triple, expected) in [
                ("aarch64-apple-darwin", "arm64-macos"),
                ("x86_64-unknown-linux-gnu", "x64-linux-gnu"),
                ("aarch64-unknown-linux-musl", "arm64-linux-musl"),
                ("x86_64-pc-windows-msvc", "x64-windows"),
                ("aarch64-linux-android", "arm64-android"),
            ] {
                assert_eq!(SystemPlatform::parse(triple).unwrap().to_string(), expected);
            }
        }
    }

    mod serde {
        use super::*;

        #[test]
        fn serializes_to_string() {
            assert_eq!(
                serde_json::to_string(&SystemPlatform::parse("arm64-linux-musl").unwrap()).unwrap(),
                "\"arm64-linux-musl\""
            );
        }

        #[test]
        fn deserializes_from_string() {
            assert_eq!(
                serde_json::from_str::<Vec<SystemPlatform>>(
                    r#"["arm64-macos", "x64-linux", "aarch64-unknown-linux-musl"]"#
                )
                .unwrap(),
                vec![
                    SystemPlatform::new(SystemOS::MacOS, SystemArch::Arm64),
                    SystemPlatform::new(SystemOS::Linux, SystemArch::X64),
                    SystemPlatform::new(SystemOS::Linux, SystemArch::Arm64)
                        .with_libc(SystemLibc::Musl),
                ]
            );
        }

        #[test]
        fn errors_when_deserializing_invalid_string() {
            let error = serde_json::from_str::<SystemPlatform>(r#""x64""#).unwrap_err();

            assert!(error.to_string().contains("Invalid platform `x64`"));
        }
    }

    #[test]
    fn from_env_matches_current_system() {
        let platform = SystemPlatform::from_env();

        assert_eq!(platform.os, SystemOS::from_env());
        assert_eq!(platform.arch, SystemArch::from_env());
        assert_eq!(platform.libc, SystemLibc::detect(platform.os));
    }
}

mod system_libc {
    use super::*;

    #[test]
    fn deserializes_known_values() {
        assert_eq!(
            serde_json::from_str::<SystemLibc>(r#""gnu""#).unwrap(),
            SystemLibc::Gnu
        );
        assert_eq!(
            serde_json::from_str::<SystemLibc>(r#""glibc""#).unwrap(),
            SystemLibc::Gnu
        );
        assert_eq!(
            serde_json::from_str::<SystemLibc>(r#""musl""#).unwrap(),
            SystemLibc::Musl
        );
        assert_eq!(
            serde_json::from_str::<SystemLibc>(r#""unknown""#).unwrap(),
            SystemLibc::Unknown
        );
    }

    // A newer version of proto may send a libc that this version doesn't support
    #[test]
    fn deserializes_unsupported_values_as_unknown() {
        for value in [r#""msvc""#, r#""bionic""#, r#""libsystem""#] {
            assert_eq!(
                serde_json::from_str::<SystemLibc>(value).unwrap(),
                SystemLibc::Unknown,
                "{value}"
            );
        }
    }
}

mod system_abi {
    use super::*;

    #[test]
    fn deserializes_unsupported_values_as_unknown() {
        assert_eq!(
            serde_json::from_str::<SystemABI>(r#""eabihf""#).unwrap(),
            SystemABI::Eabihf
        );
        assert_eq!(
            serde_json::from_str::<SystemABI>(r#""gnuabi64""#).unwrap(),
            SystemABI::Unknown
        );
    }
}
