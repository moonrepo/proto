use ::semver::{BuildMetadata, Prerelease, Version};
use version_spec::{SemVer, SpecError, is_semver};

mod semver {
    use super::*;

    #[test]
    fn matches() {
        assert!(is_semver("0.0.0"));
        assert!(is_semver("1.2.3"));
        assert!(is_semver("10.20.30"));

        // pre
        assert!(is_semver("1.2.3-0"));
        assert!(is_semver("1.2.3-alpha"));
        assert!(is_semver("1.2.3-alpha.1"));
        assert!(is_semver("1.2.3-rc.1.2"));

        // build
        assert!(is_semver("1.2.3+build"));
        assert!(is_semver("1.2.3+build.123"));

        // pre + build
        assert!(is_semver("1.2.3-beta.1+exp.sha.5114f85"));
    }

    #[test]
    fn matches_scoped() {
        assert!(is_semver("node-1.2.3"));
        assert!(is_semver("foo-bar-1.2.3"));
        assert!(is_semver("foo_bar-1.2.3"));
        assert!(is_semver("graalvm-ce-21.0.2"));
        assert!(is_semver("a1-1.2.3"));

        // pre + build
        assert!(is_semver("node-1.2.3-alpha.1"));
        assert!(is_semver("node-1.2.3+build"));
        assert!(is_semver("node-1.2.3-alpha.1+build"));
    }

    #[test]
    fn doesnt_match() {
        assert!(!is_semver(""));
        assert!(!is_semver("1"));
        assert!(!is_semver("1.2"));
        assert!(!is_semver("1.2.3.4"));
        assert!(!is_semver("v1.2.3"));

        // invalid separators
        assert!(!is_semver("1-2-3"));
        assert!(!is_semver("1x2x3"));
        assert!(!is_semver("1.2-3"));

        // aliases
        assert!(!is_semver("latest"));
        assert!(!is_semver("node"));

        // scoped partials
        assert!(!is_semver("node-1"));
        assert!(!is_semver("node-1.2"));
    }

    #[test]
    fn parse() {
        let ver = SemVer::parse("1.2.3").unwrap();

        assert_eq!(ver.0, Version::new(1, 2, 3));
        assert_eq!(ver.1, None);
        assert_eq!(ver.to_string(), "1.2.3");

        // pre + build
        let ver = SemVer::parse("1.2.3-alpha.1+build.5").unwrap();

        assert_eq!(
            ver.0,
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                pre: Prerelease::new("alpha.1").unwrap(),
                build: BuildMetadata::new("build.5").unwrap(),
            }
        );
        assert_eq!(ver.1, None);
        assert_eq!(ver.to_string(), "1.2.3-alpha.1+build.5");
    }

    #[test]
    fn parse_with_scope() {
        let ver = SemVer::parse("node-1.2.3").unwrap();

        assert_eq!(ver.0, Version::new(1, 2, 3));
        assert_eq!(ver.1, Some("node".to_owned()));
        assert_eq!(ver.to_string(), "node-1.2.3");

        // multi-part scope
        let ver = SemVer::parse("graalvm-ce-21.0.2").unwrap();

        assert_eq!(ver.0, Version::new(21, 0, 2));
        assert_eq!(ver.1, Some("graalvm-ce".to_owned()));
        assert_eq!(ver.to_string(), "graalvm-ce-21.0.2");

        // underscore scope
        let ver = SemVer::parse("foo_bar-1.2.3").unwrap();

        assert_eq!(ver.0, Version::new(1, 2, 3));
        assert_eq!(ver.1, Some("foo_bar".to_owned()));
        assert_eq!(ver.to_string(), "foo_bar-1.2.3");

        // pre + build
        let ver = SemVer::parse("node-1.2.3-alpha.1+build.5").unwrap();

        assert_eq!(
            ver.0,
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                pre: Prerelease::new("alpha.1").unwrap(),
                build: BuildMetadata::new("build.5").unwrap(),
            }
        );
        assert_eq!(ver.1, Some("node".to_owned()));
        assert_eq!(ver.to_string(), "node-1.2.3-alpha.1+build.5");
    }

    #[test]
    fn parse_prefers_version_over_scope() {
        // A pre-release is not extracted as a scope
        let ver = SemVer::parse("1.2.3-alpha").unwrap();

        assert_eq!(ver.0, Version::parse("1.2.3-alpha").unwrap());
        assert_eq!(ver.1, None);
        assert_eq!(ver.to_string(), "1.2.3-alpha");
    }

    #[test]
    fn parse_scope_with_trailing_dash() {
        let ver = SemVer::parse("foo--1.2.3").unwrap();

        assert_eq!(ver.0, Version::new(1, 2, 3));
        assert_eq!(ver.1, Some("foo-".to_owned()));
        assert_eq!(ver.to_string(), "foo--1.2.3");
    }

    #[test]
    fn serializes_to_string() {
        for value in ["1.2.3", "1.2.3-alpha.1+build.5", "node-1.2.3"] {
            let ver = SemVer::parse(value).unwrap();

            assert_eq!(serde_json::to_string(&ver).unwrap(), format!("\"{value}\""));
            assert_eq!(
                serde_json::from_str::<SemVer>(&format!("\"{value}\"")).unwrap(),
                ver
            );
        }
    }

    #[test]
    fn parse_errors() {
        assert!(matches!(
            SemVer::parse("1.2"),
            Err(SpecError::InvalidSemverFormat)
        ));
        assert!(matches!(
            SemVer::parse("abc"),
            Err(SpecError::InvalidSemverFormat)
        ));
        assert!(matches!(
            SemVer::parse(""),
            Err(SpecError::InvalidSemverFormat)
        ));

        // matches the regex but not a valid semver (leading zero)
        assert!(matches!(SemVer::parse("01.2.3"), Err(SpecError::Semver(_))));
    }
}
