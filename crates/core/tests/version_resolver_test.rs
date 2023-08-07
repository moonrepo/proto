use proto_core::{resolve_version, VersionType};
use semver::Version;
use std::collections::BTreeMap;

mod version_resolver {
    use super::*;

    fn create_versions() -> Vec<Version> {
        vec![
            Version::new(1, 0, 0),
            Version::new(1, 2, 3),
            Version::new(1, 1, 1),
            Version::new(1, 5, 9),
            Version::new(1, 10, 5),
            Version::new(4, 5, 6),
            Version::new(7, 8, 9),
            Version::new(8, 0, 0),
            Version::new(10, 0, 0),
        ]
    }

    fn create_aliases() -> BTreeMap<String, VersionType> {
        BTreeMap::from_iter([
            (
                "latest".into(),
                VersionType::Version(Version::new(10, 0, 0)),
            ),
            ("stable".into(), VersionType::Alias("latest".into())),
            (
                "no-version".into(),
                VersionType::Version(Version::new(20, 0, 0)),
            ),
            ("no-alias".into(), VersionType::Alias("missing".into())),
        ])
    }

    #[test]
    fn resolves_aliases() {
        let versions = create_versions();
        let aliases = create_aliases();

        assert_eq!(
            resolve_version(
                &VersionType::Alias("latest".into()),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(10, 0, 0)
        );

        assert_eq!(
            resolve_version(
                &VersionType::Alias("stable".into()),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(10, 0, 0)
        );
    }

    #[test]
    #[should_panic(expected = "Failed to resolve a semantic version for unknown.")]
    fn errors_unknown_alias() {
        let versions = create_versions();
        let aliases = create_aliases();

        resolve_version(
            &VersionType::Alias("unknown".into()),
            &versions.iter().collect::<Vec<_>>(),
            &aliases,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Failed to resolve a semantic version for missing.")]
    fn errors_missing_alias_from_alias() {
        let versions = create_versions();
        let aliases = create_aliases();

        resolve_version(
            &VersionType::Alias("no-alias".into()),
            &versions.iter().collect::<Vec<_>>(),
            &aliases,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Failed to resolve a semantic version for 20.0.0.")]
    fn errors_missing_version_from_alias() {
        let versions = create_versions();
        let aliases = create_aliases();

        resolve_version(
            &VersionType::Alias("no-version".into()),
            &versions.iter().collect::<Vec<_>>(),
            &aliases,
        )
        .unwrap();
    }

    #[test]
    fn resolves_versions() {
        let versions = create_versions();
        let aliases = create_aliases();

        assert_eq!(
            resolve_version(
                &VersionType::Version(Version::new(1, 10, 5)),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(1, 10, 5)
        );

        assert_eq!(
            resolve_version(
                &VersionType::Version(Version::new(8, 0, 0)),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );
    }

    #[test]
    fn resolves_partial_versions() {
        let versions = create_versions();
        let aliases = create_aliases();

        assert_eq!(
            resolve_version(
                &VersionType::parse("1.2").unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(1, 2, 3)
        );

        assert_eq!(
            resolve_version(
                &VersionType::parse("1.0").unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(1, 0, 0)
        );

        assert_eq!(
            resolve_version(
                &VersionType::parse("1").unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(1, 10, 5)
        );
    }

    #[test]
    fn removes_v_prefix() {
        let versions = create_versions();
        let aliases = create_aliases();

        assert_eq!(
            resolve_version(
                &VersionType::parse("v8.0.0").unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );

        assert_eq!(
            resolve_version(
                &VersionType::parse("V8").unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );
    }

    #[test]
    #[should_panic(expected = "Failed to resolve a semantic version for 20.0.0.")]
    fn errors_unknown_version() {
        let versions = create_versions();
        let aliases = create_aliases();

        resolve_version(
            &VersionType::Version(Version::new(20, 0, 0)),
            &versions.iter().collect::<Vec<_>>(),
            &aliases,
        )
        .unwrap();
    }

    #[test]
    fn resolves_req() {
        let versions = create_versions();
        let aliases = create_aliases();

        assert_eq!(
            resolve_version(
                &VersionType::parse("^8").unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );

        assert_eq!(
            resolve_version(
                &VersionType::parse("~1.1").unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(1, 1, 1)
        );

        assert_eq!(
            resolve_version(
                &VersionType::parse(">1 <10").unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );

        assert_eq!(
            resolve_version(
                &VersionType::parse(">1, <10").unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );

        // Highest match
        assert_eq!(
            resolve_version(
                &VersionType::parse("^1").unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(1, 10, 5)
        );

        // Star (latest)
        assert_eq!(
            resolve_version(
                &VersionType::parse("*").unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(10, 0, 0)
        );
    }

    #[test]
    #[should_panic(expected = "Failed to resolve a semantic version for ^20.")]
    fn errors_no_req_match() {
        let versions = create_versions();
        let aliases = create_aliases();

        resolve_version(
            &VersionType::parse("^20").unwrap(),
            &versions.iter().collect::<Vec<_>>(),
            &aliases,
        )
        .unwrap();
    }

    #[test]
    fn resolves_req_any() {
        let versions = create_versions();
        let aliases = create_aliases();

        assert_eq!(
            resolve_version(
                &VersionType::parse("^1 || ^6 || ^8").unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );
    }

    #[test]
    #[should_panic(expected = "Failed to resolve a semantic version for ^9 || ^5 || ^3.")]
    fn errors_no_req_any_match() {
        let versions = create_versions();
        let aliases = create_aliases();

        resolve_version(
            &VersionType::parse("^3 || ^5 || ^9").unwrap(),
            &versions.iter().collect::<Vec<_>>(),
            &aliases,
        )
        .unwrap();
    }

    #[test]
    fn handles_gt_lt_with_space() {
        let versions = create_versions();
        let aliases = create_aliases();

        for req in [">= 1.5.9", "> 1.5.0", ">= 1.2", "> 1.2", "< 1.2", "<= 1.2"] {
            resolve_version(
                &VersionType::parse(req).unwrap(),
                &versions.iter().collect::<Vec<_>>(),
                &aliases,
            )
            .unwrap();
        }
    }
}
