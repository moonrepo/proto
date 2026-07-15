use compact_str::CompactString;
use semver::Version;
use version_spec::{CalVer, SemVer, VersionSpec};

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
            VersionSpec::Semantic(SemVer(Version::new(1, 2, 3), None))
        );
        assert_eq!(
            VersionSpec::parse("1.2.3").unwrap(),
            VersionSpec::Semantic(SemVer(Version::new(1, 2, 3), None))
        );
        assert_eq!(
            VersionSpec::parse("1.2.3-0").unwrap(),
            VersionSpec::Semantic(SemVer(Version::parse("1.2.3-0").unwrap(), None))
        );
        assert_eq!(
            VersionSpec::parse("1.2.3-alpha").unwrap(),
            VersionSpec::Semantic(SemVer(Version::parse("1.2.3-alpha").unwrap(), None))
        );
        assert_eq!(
            VersionSpec::parse("1.2.3-alpha.1").unwrap(),
            VersionSpec::Semantic(SemVer(Version::parse("1.2.3-alpha.1").unwrap(), None))
        );

        // calver
        assert_eq!(
            VersionSpec::parse("2024-02").unwrap(),
            VersionSpec::Calendar(CalVer(Version::new(2024, 2, 0), None))
        );
        assert_eq!(
            VersionSpec::parse("2024-2-26").unwrap(),
            VersionSpec::Calendar(CalVer(Version::new(2024, 2, 26), None))
        );
    }

    #[test]
    #[should_panic(expected = "UnknownResolvedFormat")]
    fn error_invalid_char() {
        VersionSpec::parse("%").unwrap();
    }

    #[test]
    fn compares_against_version() {
        let version = Version::new(1, 2, 3);

        assert_eq!(
            VersionSpec::Semantic(SemVer(version.clone(), None)),
            version
        );
        assert_ne!(
            VersionSpec::Semantic(SemVer(version.clone(), Some("scope".into()))),
            version
        );

        let version = Version::new(2024, 2, 26);

        assert_eq!(
            VersionSpec::Calendar(CalVer(version.clone(), None)),
            version
        );
        assert_ne!(
            VersionSpec::Calendar(CalVer(version.clone(), Some("scope".into()))),
            version
        );
    }
}
