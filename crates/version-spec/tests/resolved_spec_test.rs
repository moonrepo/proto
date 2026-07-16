use compact_str::CompactString;
use version_spec::{Version, VersionKind, VersionSpec};

mod resolved_spec {
    use super::*;

    #[test]
    fn canary() {
        assert_eq!(VersionSpec::parse("canary").unwrap(), VersionSpec::Canary);
    }

    #[test]
    fn aliases() {
        assert_eq!(
            VersionSpec::parse("latest").unwrap(),
            VersionSpec::Alias(CompactString::new("latest"))
        );
        assert_eq!(
            VersionSpec::parse("stable").unwrap(),
            VersionSpec::Alias(CompactString::new("stable"))
        );
        assert_eq!(
            VersionSpec::parse("legacy-2023").unwrap(),
            VersionSpec::Alias(CompactString::new("legacy-2023"))
        );
        assert_eq!(
            VersionSpec::parse("future/202x").unwrap(),
            VersionSpec::Alias(CompactString::new("future/202x"))
        );
    }

    #[test]
    fn versions() {
        assert_eq!(
            VersionSpec::parse("v1.2.3").unwrap(),
            VersionSpec::Version(Version::semantic(1, 2, 3))
        );
        assert_eq!(
            VersionSpec::parse("1.2.3").unwrap(),
            VersionSpec::Version(Version::semantic(1, 2, 3))
        );
        assert_eq!(
            VersionSpec::parse("1.2.3-0").unwrap(),
            VersionSpec::Version(Version {
                prerelease: Some("0".into()),
                ..Version::semantic(1, 2, 3)
            })
        );
        assert_eq!(
            VersionSpec::parse("1.2.3-alpha").unwrap(),
            VersionSpec::Version(Version {
                prerelease: Some("alpha".into()),
                ..Version::semantic(1, 2, 3)
            })
        );
        assert_eq!(
            VersionSpec::parse("1.2.3-alpha.1").unwrap(),
            VersionSpec::Version(Version {
                prerelease: Some("alpha.1".into()),
                ..Version::semantic(1, 2, 3)
            })
        );

        // calver
        assert_eq!(
            VersionSpec::parse("2024-02").unwrap(),
            VersionSpec::Version(Version {
                kind: VersionKind::Calendar,
                major: 2024,
                minor: 2,
                ..Default::default()
            })
        );
        assert_eq!(
            VersionSpec::parse("2024-2-26").unwrap(),
            VersionSpec::Version(Version::calendar(2024, 2, 26))
        );
    }

    #[test]
    fn scoped_versions() {
        assert_eq!(
            VersionSpec::parse("node-1.2.3").unwrap(),
            VersionSpec::Version(Version {
                scope: Some("node".into()),
                ..Version::semantic(1, 2, 3)
            })
        );
        assert_eq!(
            VersionSpec::parse("v8-1.2.3").unwrap(),
            VersionSpec::Version(Version {
                scope: Some("v8".into()),
                ..Version::semantic(1, 2, 3)
            })
        );

        // calver
        assert_eq!(
            VersionSpec::parse("node-2024-02").unwrap(),
            VersionSpec::Version(Version {
                kind: VersionKind::Calendar,
                scope: Some("node".into()),
                major: 2024,
                minor: 2,
                ..Default::default()
            })
        );
        assert_eq!(
            VersionSpec::parse("foo-bar-2024-2-26").unwrap(),
            VersionSpec::Version(Version {
                scope: Some("foo-bar".into()),
                ..Version::calendar(2024, 2, 26)
            })
        );
    }

    #[test]
    fn serde_roundtrip() {
        for value in [
            "canary",
            "latest",
            "legacy-2023",
            "1.2.3",
            "1.2.3-alpha.1",
            "node-1.2.3",
            "2024-02-26",
            "node-2024-02",
        ] {
            let spec = VersionSpec::parse(value).unwrap();
            let json = serde_json::to_string(&spec).unwrap();

            assert_eq!(serde_json::from_str::<VersionSpec>(&json).unwrap(), spec);
        }
    }

    #[test]
    #[should_panic(expected = "FailedVersionParse")]
    fn error_invalid_char() {
        VersionSpec::parse("%").unwrap();
    }

    #[test]
    fn compares_against_version() {
        let version = Version::semantic(1, 2, 3);

        assert_eq!(VersionSpec::Version(version.clone()), version);
        assert_ne!(
            VersionSpec::Version(Version {
                scope: Some("scope".into()),
                ..version.clone()
            }),
            version
        );

        let version = Version::calendar(2024, 2, 26);

        assert_eq!(VersionSpec::Version(version.clone()), version);
        assert_ne!(
            VersionSpec::Version(Version {
                scope: Some("scope".into()),
                ..version.clone()
            }),
            version
        );
    }
}
