use proto_core::{
    ProtoToolConfig, ToolManifest, UnresolvedVersionSpec, Version, VersionResolver, VersionSpec,
    resolve_version,
};
use proto_pdk_api::LoadVersionsOutput;
use std::collections::BTreeMap;

mod derived_latest {
    use super::*;

    fn create_resolver(versions: &[&str]) -> VersionResolver<'static> {
        VersionResolver::from_output(LoadVersionsOutput {
            versions: versions
                .iter()
                .map(|version| VersionSpec::parse(version).unwrap())
                .collect(),
            ..Default::default()
        })
    }

    fn latest() -> UnresolvedVersionSpec {
        UnresolvedVersionSpec::Alias("latest".into())
    }

    #[test]
    fn uses_highest_version() {
        let resolver = create_resolver(&["1.0.0", "2.1.0", "2.0.0"]);

        assert_eq!(
            resolver.resolve(&latest()),
            Some(VersionSpec::parse("2.1.0").unwrap())
        );
    }

    #[test]
    fn skips_prereleases() {
        let resolver = create_resolver(&["1.0.0", "2.0.0", "3.0.0-beta.1", "3.0.0-rc.1", "canary"]);

        assert_eq!(
            resolver.aliases.get("latest").unwrap(),
            &UnresolvedVersionSpec::parse("2.0.0").unwrap()
        );
        assert_eq!(
            resolver.resolve(&latest()),
            Some(VersionSpec::parse("2.0.0").unwrap())
        );
    }

    #[test]
    fn isnt_derived_when_only_prereleases() {
        let resolver = create_resolver(&["1.0.0-alpha.1", "1.0.0-rc.1"]);

        assert!(!resolver.aliases.contains_key("latest"));
        assert_eq!(resolver.resolve(&latest()), None);
    }

    #[test]
    fn doesnt_override_plugin_latest() {
        let resolver = VersionResolver::from_output(LoadVersionsOutput {
            latest: Some(UnresolvedVersionSpec::parse("2.0.0-rc.1").unwrap()),
            versions: vec![
                VersionSpec::parse("1.0.0").unwrap(),
                VersionSpec::parse("2.0.0-rc.1").unwrap(),
            ],
            ..Default::default()
        });

        assert_eq!(
            resolver.resolve(&latest()),
            Some(VersionSpec::parse("2.0.0-rc.1").unwrap())
        );
    }
}

mod alias_targets {
    use super::*;

    fn create_resolver(
        versions: &[&str],
        aliases: &[(&str, &str)],
        latest: Option<&str>,
    ) -> VersionResolver<'static> {
        VersionResolver::from_output(LoadVersionsOutput {
            aliases: aliases
                .iter()
                .map(|(alias, spec)| {
                    (
                        alias.to_string(),
                        UnresolvedVersionSpec::parse(spec).unwrap(),
                    )
                })
                .collect(),
            latest: latest.map(|spec| UnresolvedVersionSpec::parse(spec).unwrap()),
            versions: versions
                .iter()
                .map(|version| VersionSpec::parse(version).unwrap())
                .collect(),
            ..Default::default()
        })
    }

    fn alias(name: &str) -> UnresolvedVersionSpec {
        UnresolvedVersionSpec::Alias(name.into())
    }

    fn version(value: &str) -> VersionSpec {
        VersionSpec::parse(value).unwrap()
    }

    #[test]
    fn resolves_unlisted_version() {
        let resolver = create_resolver(
            &["0.14.0", "0.15.0"],
            &[("master", "0.16.0-dev.1234")],
            None,
        );

        assert_eq!(
            resolver.resolve(&alias("master")),
            Some(version("0.16.0-dev.1234"))
        );
    }

    #[test]
    fn adds_unlisted_version_to_list_in_order() {
        let resolver = create_resolver(
            &["0.14.0", "0.15.0"],
            &[("master", "0.16.0-dev.1234")],
            None,
        );

        assert_eq!(
            resolver.versions,
            vec![
                version("0.16.0-dev.1234"),
                version("0.15.0"),
                version("0.14.0")
            ]
        );
    }

    #[test]
    fn resolves_unlisted_version_from_alias_chain() {
        let resolver = create_resolver(
            &["0.15.0"],
            &[("master", "0.16.0-dev.1234"), ("nightly", "master")],
            None,
        );

        assert_eq!(
            resolver.resolve(&alias("nightly")),
            Some(version("0.16.0-dev.1234"))
        );
    }

    #[test]
    fn resolves_unlisted_plugin_latest() {
        let resolver = create_resolver(&["1.0.0", "2.0.0"], &[], Some("2.1.0"));

        assert_eq!(resolver.resolve(&alias("latest")), Some(version("2.1.0")));
    }

    #[test]
    fn doesnt_duplicate_versions() {
        let resolver = create_resolver(
            &["1.0.0", "2.0.0"],
            &[
                ("stable", "2.0.0"),
                ("next", "3.0.0-rc.1"),
                ("rc", "3.0.0-rc.1"),
            ],
            Some("2.0.0"),
        );

        assert_eq!(
            resolver.versions,
            vec![version("3.0.0-rc.1"), version("2.0.0"), version("1.0.0")]
        );
    }

    #[test]
    fn doesnt_derive_latest_from_unlisted_prerelease() {
        let resolver = create_resolver(
            &["0.14.0", "0.15.0"],
            &[("master", "0.16.0-dev.1234")],
            None,
        );

        assert_eq!(resolver.resolve(&alias("latest")), Some(version("0.15.0")));
    }

    #[test]
    fn resolves_unlisted_requirement_against_list() {
        let resolver = create_resolver(
            &["1.2.0", "1.2.5", "1.3.0"],
            &[("lts", "~1.2"), ("legacy", "^1.4")],
            None,
        );

        assert_eq!(resolver.resolve(&alias("lts")), Some(version("1.2.5")));
        assert_eq!(resolver.resolve(&alias("legacy")), None);
        assert_eq!(
            resolver.versions,
            vec![version("1.3.0"), version("1.2.5"), version("1.2.0")]
        );
    }
}

mod version_resolver {
    use super::*;

    fn create_versions() -> Vec<VersionSpec> {
        vec![
            VersionSpec::Version(Version::new(1, 0, 0)),
            VersionSpec::Version(Version::new(1, 2, 3)),
            VersionSpec::Version(Version::new(1, 1, 1)),
            VersionSpec::Version(Version::new(1, 5, 9)),
            VersionSpec::Version(Version::new(1, 10, 5)),
            VersionSpec::Version(Version::new(4, 5, 6)),
            VersionSpec::Version(Version::new(7, 8, 9)),
            VersionSpec::Version(Version::new(8, 0, 0)),
            VersionSpec::Version(Version::new(10, 0, 0)),
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

        manifest
            .installed_versions
            .insert(VersionSpec::parse("3.0.0").unwrap());
        manifest
            .installed_versions
            .insert(VersionSpec::parse("3.3.3").unwrap());

        manifest
    }

    fn create_tool_config() -> ProtoToolConfig {
        let mut config = ProtoToolConfig::default();

        config.aliases.insert(
            "latest-manifest".into(),
            UnresolvedVersionSpec::Version(Version::new(8, 0, 0)).into(),
        );
        config.aliases.insert(
            "stable-manifest".into(),
            UnresolvedVersionSpec::Alias("stable".into()).into(),
        );

        config
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
    fn resolves_aliases_from_config() {
        let versions = create_versions();
        let aliases = create_aliases();
        let manifest = create_manifest();
        let config = create_tool_config();

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::Alias("latest-manifest".into()),
                &versions,
                &aliases,
                Some(&manifest),
                Some(&config),
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
                Some(&config),
            )
            .unwrap(),
            Version::new(10, 0, 0)
        );
    }

    #[test]
    fn resolves_without_manifest_but_with_config() {
        let mut manifest = ToolManifest::default();
        manifest
            .installed_versions
            .insert(VersionSpec::parse("1.2.3").unwrap());

        let mut config = ProtoToolConfig::default();
        config.aliases.insert(
            "work".into(),
            UnresolvedVersionSpec::parse("^1.2").unwrap().into(),
        );

        let mut resolver = VersionResolver::default();
        resolver.versions = create_versions();
        resolver.aliases = create_aliases();
        resolver.with_manifest(&manifest);
        resolver.with_config(&config);

        let work = UnresolvedVersionSpec::Alias("work".into());
        let req = UnresolvedVersionSpec::parse("^1.2").unwrap();

        // Prefers the installed version
        assert_eq!(resolver.resolve(&work).unwrap(), Version::new(1, 2, 3));
        assert_eq!(resolver.resolve(&req).unwrap(), Version::new(1, 2, 3));

        // Ignores the installed version, but still applies config aliases
        assert_eq!(
            resolver.resolve_without_manifest(&work).unwrap(),
            Version::new(1, 10, 5)
        );
        assert_eq!(
            resolver.resolve_without_manifest(&req).unwrap(),
            Version::new(1, 10, 5)
        );
    }

    #[test]
    #[should_panic]
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
    #[should_panic]
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
    #[should_panic]
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
    fn resolves_versions_from_config() {
        let versions = create_versions();
        let aliases = create_aliases();
        let manifest = create_manifest();
        let config = create_tool_config();

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::Version(Version::new(3, 0, 0)),
                &versions,
                &aliases,
                Some(&manifest),
                Some(&config),
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
    fn resolves_partial_versions_with_config() {
        let versions = create_versions();
        let aliases = create_aliases();
        let manifest = create_manifest();
        let config = create_tool_config();

        assert_eq!(
            resolve_version(
                &UnresolvedVersionSpec::parse("3.3").unwrap(),
                &versions,
                &aliases,
                Some(&manifest),
                Some(&config),
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
                Some(&config),
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
    #[should_panic]
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
    #[should_panic]
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
    #[should_panic]
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
