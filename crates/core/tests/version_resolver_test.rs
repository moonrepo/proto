use proto_core::{resolve_version, ToolManifest, UnresolvedVersionSpec, VersionSpec};
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

    fn create_aliases() -> BTreeMap<String, UnresolvedVersionSpec> {
        BTreeMap::from_iter([
            (
                "latest".into(),
                UnresolvedVersionSpec::Version(Version::new(10, 0, 0)),
            ),
            (
                "stable".into(),
                UnresolvedVersionSpec::Alias("latest".into()),
            ),
            (
                "no-version".into(),
                UnresolvedVersionSpec::Version(Version::new(20, 0, 0)),
            ),
            (
                "no-alias".into(),
                UnresolvedVersionSpec::Alias("missing".into()),
            ),
        ])
    }

    fn create_manifest() -> ToolManifest {
        let mut manifest = ToolManifest::default();

        manifest.aliases.insert(
            "latest-manifest".into(),
            UnresolvedVersionSpec::Version(Version::new(8, 0, 0)),
        );
        manifest.aliases.insert(
            "stable-manifest".into(),
            UnresolvedVersionSpec::Alias("stable".into()),
        );

        manifest
            .installed_versions
            .insert(VersionSpec::parse("3.0.0").unwrap());
        manifest
            .installed_versions
            .insert(VersionSpec::parse("3.3.3").unwrap());

        manifest
    }

    #[test]
    fn resolves_aliases() {
        let versions = create_versions();
        let aliases = create_aliases();

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::Alias("latest".into()),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(10, 0, 0)
        );

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::Alias("stable".into()),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(10, 0, 0)
        );
    }

    #[test]
    fn resolves_aliases_from_manifest() {
        let versions = create_versions();
        let aliases = create_aliases();
        let manifest = create_manifest();

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::Alias("latest-manifest".into()),
                &versions,
                &aliases,
                Some(&manifest),
                None,
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::Alias("stable-manifest".into()),
                &versions,
                &aliases,
                Some(&manifest),
                None,
            )
            .unwrap(),
            Version::new(10, 0, 0)
        );
    }

    #[test]
    #[should_panic(expected = "Failed to resolve unknown to a valid supported version.")]
    fn errors_unknown_alias() {
        let versions = create_versions();
        let aliases = create_aliases();

        resolve_version(
            &UnresolvedVersionSpec::Alias("unknown".into()),
            &versions,
            &aliases,
            None,
            None,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Failed to resolve missing to a valid supported version.")]
    fn errors_missing_alias_from_alias() {
        let versions = create_versions();
        let aliases = create_aliases();

        resolve_version(
            &UnresolvedVersionSpec::Alias("no-alias".into()),
            &versions,
            &aliases,
            None,
            None,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Failed to resolve 20.0.0 to a valid supported version.")]
    fn errors_missing_version_from_alias() {
        let versions = create_versions();
        let aliases = create_aliases();

        resolve_version(
            &UnresolvedVersionSpec::Alias("no-version".into()),
            &versions,
            &aliases,
            None,
            None,
        )
        .unwrap();
    }

    #[test]
    fn resolves_versions() {
        let versions = create_versions();
        let aliases = create_aliases();

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::Version(Version::new(1, 10, 5)),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(1, 10, 5)
        );

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::Version(Version::new(8, 0, 0)),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );
    }

    #[test]
    fn resolves_versions_from_manifest() {
        let versions = create_versions();
        let aliases = create_aliases();
        let manifest = create_manifest();

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::Version(Version::new(3, 0, 0)),
                &versions,
                &aliases,
                Some(&manifest),
                None,
            )
            .unwrap(),
            Version::new(3, 0, 0)
        );
    }

    #[test]
    fn resolves_partial_versions() {
        let versions = create_versions();
        let aliases = create_aliases();

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("1.2").unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(1, 2, 3)
        );

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("1.0").unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(1, 0, 0)
        );

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("1").unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(1, 10, 5)
        );
    }

    #[test]
    fn resolves_partial_versions_with_manifest() {
        let versions = create_versions();
        let aliases = create_aliases();
        let manifest = create_manifest();

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("3.3").unwrap(),
                &versions,
                &aliases,
                Some(&manifest),
                None,
            )
            .unwrap(),
            Version::new(3, 3, 3)
        );

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("3").unwrap(),
                &versions,
                &aliases,
                Some(&manifest),
                None,
            )
            .unwrap(),
            Version::new(3, 3, 3)
        );
    }

    #[test]
    fn removes_v_prefix() {
        let versions = create_versions();
        let aliases = create_aliases();

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("v8.0.0").unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("V8").unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );
    }

    #[test]
    #[should_panic(expected = "Failed to resolve 20.0.0 to a valid supported version.")]
    fn errors_unknown_version() {
        let versions = create_versions();
        let aliases = create_aliases();

        resolve_version(
            &UnresolvedVersionSpec::Version(Version::new(20, 0, 0)),
            &versions,
            &aliases,
            None,
            None,
        )
        .unwrap();
    }

    #[test]
    fn resolves_req() {
        let versions = create_versions();
        let aliases = create_aliases();

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("^8").unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("~1.1").unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(1, 1, 1)
        );

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse(">1 <10").unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse(">1, <10").unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );

        // Highest match
        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("^1").unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(1, 10, 5)
        );

        // Star (latest)
        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("*").unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(10, 0, 0)
        );
    }

    #[test]
    #[should_panic(expected = "Failed to resolve ^20 to a valid supported version.")]
    fn errors_no_req_match() {
        let versions = create_versions();
        let aliases = create_aliases();

        resolve_version(
            &UnresolvedVersionSpec::parse("^20").unwrap(),
            &versions,
            &aliases,
            None,
            None,
        )
        .unwrap();
    }

    #[test]
    fn resolves_req_any() {
        let versions = create_versions();
        let aliases = create_aliases();

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("^1 || ^6 || ^8").unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap(),
            Version::new(8, 0, 0)
        );
    }

    #[test]
    #[should_panic(expected = "Failed to resolve ^9 || ^5 || ^3 to a valid supported version.")]
    fn errors_no_req_any_match() {
        let versions = create_versions();
        let aliases = create_aliases();

        resolve_version(
            &UnresolvedVersionSpec::parse("^3 || ^5 || ^9").unwrap(),
            &versions,
            &aliases,
            None,
            None,
        )
        .unwrap();
    }

    #[test]
    fn handles_gt_lt_with_space() {
        let versions = create_versions();
        let aliases = create_aliases();

        for req in [">= 1.5.9", "> 1.5.0", ">= 1.2", "> 1.2", "< 1.2", "<= 1.2"] {
            resolve_version(
                &UnresolvedVersionSpec::parse(req).unwrap(),
                &versions,
                &aliases,
                None,
                None,
            )
            .unwrap();
        }
    }
}
