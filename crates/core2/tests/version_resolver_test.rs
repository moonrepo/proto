use proto_core::{resolve_version, AliasOrVersion, VersionType};
use semver::{Version, VersionReq};
use std::collections::BTreeMap;
use std::str::FromStr;

mod version_resolver {
    use super::*;

    fn create_versions() -> Vec<Version> {
        vec![
            Version::new(1, 0, 0),
            Version::new(1, 2, 3),
            Version::new(1, 1, 1),
            Version::new(1, 10, 5),
            Version::new(4, 5, 6),
            Version::new(7, 8, 9),
            Version::new(8, 0, 0),
            Version::new(10, 0, 0),
        ]
    }

    fn create_aliases() -> BTreeMap<String, AliasOrVersion> {
        BTreeMap::from_iter([
            (
                "latest".into(),
                AliasOrVersion::Version(Version::new(10, 0, 0)),
            ),
            ("stable".into(), AliasOrVersion::Alias("latest".into())),
            (
                "no-version".into(),
                AliasOrVersion::Version(Version::new(20, 0, 0)),
            ),
            ("no-alias".into(), AliasOrVersion::Alias("missing".into())),
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
                &VersionType::ReqAll(VersionReq::parse("^8").unwrap()),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );

        assert_eq!(
            resolve_version(
                &VersionType::ReqAll(VersionReq::parse("~1.1").unwrap()),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(1, 1, 1)
        );

        assert_eq!(
            resolve_version(
                &VersionType::ReqAll(VersionReq::parse(">1, <10").unwrap()),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );

        // Highest match
        assert_eq!(
            resolve_version(
                &VersionType::ReqAll(VersionReq::parse("^1").unwrap()),
                &versions.iter().collect::<Vec<_>>(),
                &aliases
            )
            .unwrap(),
            Version::new(1, 10, 5)
        );

        // Star (latest)
        assert_eq!(
            resolve_version(
                &VersionType::ReqAll(VersionReq::parse("*").unwrap()),
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
            &VersionType::ReqAll(VersionReq::parse("^20").unwrap()),
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
                &VersionType::from_str("^1 || ^6 || ^8").unwrap(),
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
            &VersionType::from_str("^3 || ^5 || ^9").unwrap(),
            &versions.iter().collect::<Vec<_>>(),
            &aliases,
        )
        .unwrap();
    }
}
