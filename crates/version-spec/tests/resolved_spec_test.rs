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
            VersionSpec::Alias("latest".to_owned())
        );
        assert_eq!(
            VersionSpec::parse("stable").unwrap(),
            VersionSpec::Alias("stable".to_owned())
        );
        assert_eq!(
            VersionSpec::parse("legacy-2023").unwrap(),
            VersionSpec::Alias("legacy-2023".to_owned())
        );
        assert_eq!(
            VersionSpec::parse("future/202x").unwrap(),
            VersionSpec::Alias("future/202x".to_owned())
        );
    }

    #[test]
    fn versions() {
        assert_eq!(
            VersionSpec::parse("v1.2.3").unwrap(),
            VersionSpec::Semantic(SemVer(Version::new(1, 2, 3)))
        );
        assert_eq!(
            VersionSpec::parse("1.2.3").unwrap(),
            VersionSpec::Semantic(SemVer(Version::new(1, 2, 3)))
        );

        // calver
        assert_eq!(
            VersionSpec::parse("2024-02").unwrap(),
            VersionSpec::Calendar(CalVer(Version::new(2024, 2, 0)))
        );
        assert_eq!(
            VersionSpec::parse("2024-2-26").unwrap(),
            VersionSpec::Calendar(CalVer(Version::new(2024, 2, 26)))
        );
    }

    #[test]
    #[should_panic(expected = "unexpected end of input while parsing minor version number")]
    fn error_missing_patch() {
        VersionSpec::parse("1.2").unwrap();
    }

    #[test]
    #[should_panic(expected = "unexpected end of input while parsing major version number")]
    fn error_missing_minor() {
        VersionSpec::parse("1").unwrap();
    }

    #[test]
    #[should_panic(expected = "unexpected character '%' while parsing major version number")]
    fn error_invalid_char() {
        VersionSpec::parse("%").unwrap();
    }
}
