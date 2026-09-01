use proto_core::{
    ProtoConfig, ProtoEnvironment, Tool, ToolContext, ToolSpec, flow::resolve::Resolver,
    load_tool_from_locator,
};
use starbase_sandbox::create_empty_sandbox;
use std::path::Path;
use version_spec::{UnresolvedVersionSpec, VersionSpec};

async fn create_node(_root: &Path) -> Tool {
    load_tool_from_locator(
        ToolContext::parse("node").unwrap(),
        ProtoEnvironment::new().unwrap(),
        ProtoConfig::default()
            .builtin_plugins()
            .tools
            .get("node")
            .unwrap(),
    )
    .await
    .unwrap()
}

// The mocked tool implements `resolve_version`, mapping the aliases
// "stable" -> 5.0.0, "unstable" -> 6.0.0-rc.1, and "legacy" -> 4.10.15,
// while `load_versions` returns 1.0.0 through 5.10.15 plus 6.0.0 pre-releases
async fn create_mocked_tool(root: &Path) -> Tool {
    load_tool_from_locator(
        ToolContext::parse("protostar").unwrap(),
        ProtoEnvironment::new_testing(root).unwrap(),
        ProtoConfig::default()
            .builtin_plugins()
            .tools
            .get("protostar")
            .unwrap(),
    )
    .await
    .unwrap()
}

// Seed the remote versions cache for a scope, so that `load_versions` reads
// from it instead of calling the plugin, which lets us control the alias map
fn seed_remote_versions(tool: &Tool, scope: &str, contents: &str) {
    let dir = &tool.inventory.dir;

    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(format!("remote-versions-{scope}.json")), contents).unwrap();
}

mod resolver {
    use super::*;

    mod resolve_version {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_already_resolved() {
            let sandbox = create_empty_sandbox();
            let tool = create_node(sandbox.path()).await;

            let mut spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());

            let result = Resolver::new(&tool)
                .resolve_version(&mut spec, false)
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("20.0.0").unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_exact_semantic_version() {
            let sandbox = create_empty_sandbox();
            let tool = create_node(sandbox.path()).await;

            let mut spec = ToolSpec::parse("20.0.0").unwrap();

            let result = Resolver::new(&tool)
                .resolve_version(&mut spec, false)
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("20.0.0").unwrap());
            assert!(spec.is_resolved());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_partial_version_to_highest() {
            let sandbox = create_empty_sandbox();
            let tool = create_node(sandbox.path()).await;

            let mut spec = ToolSpec::parse("18").unwrap();

            let result = Resolver::new(&tool)
                .resolve_version(&mut spec, false)
                .await
                .unwrap();

            // Should resolve to the highest 18.x.x
            let resolved = result.to_string();
            assert!(
                resolved.starts_with("18."),
                "Expected 18.x.x, got {resolved}"
            );
            assert!(spec.is_resolved());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_latest_alias() {
            let sandbox = create_empty_sandbox();
            let tool = create_node(sandbox.path()).await;

            let mut spec = ToolSpec::parse("latest").unwrap();

            let result = Resolver::new(&tool)
                .resolve_version(&mut spec, false)
                .await
                .unwrap();

            // Should resolve to some concrete version
            assert!(spec.is_resolved());
            let resolved = result.to_string();
            assert!(
                !resolved.is_empty() && resolved != "latest",
                "Expected concrete version, got {resolved}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn canary_short_circuits() {
            let sandbox = create_empty_sandbox();
            let tool = create_node(sandbox.path()).await;

            let mut spec = ToolSpec::new(UnresolvedVersionSpec::Canary);

            // Canary should resolve immediately even with short_circuit=false
            let result = Resolver::new(&tool)
                .resolve_version(&mut spec, false)
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::Canary);
            assert!(spec.is_resolved());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fully_qualified_short_circuits_when_enabled() {
            let sandbox = create_empty_sandbox();
            let tool = create_node(sandbox.path()).await;

            let mut spec = ToolSpec::parse("20.0.0").unwrap();

            // With short_circuit=true, should return without loading remote versions
            let result = Resolver::new(&tool)
                .resolve_version(&mut spec, true)
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("20.0.0").unwrap());
            assert!(spec.is_resolved());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_not_short_circuit_partial_versions() {
            let sandbox = create_empty_sandbox();
            let tool = create_node(sandbox.path()).await;

            let mut spec = ToolSpec::parse("20").unwrap();

            // Even with short_circuit=true, partial versions must be resolved
            let result = Resolver::new(&tool)
                .resolve_version(&mut spec, true)
                .await
                .unwrap();

            let resolved = result.to_string();
            assert!(
                resolved.starts_with("20."),
                "Expected 20.x.x, got {resolved}"
            );
            assert!(spec.is_resolved());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_alias_remapped_by_plugin() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            let mut spec = ToolSpec::parse("stable").unwrap();

            let result = Resolver::new(&tool)
                .resolve_version(&mut spec, false)
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("5.0.0").unwrap());
            assert!(spec.is_resolved());
            assert_eq!(
                spec.get_resolved_version(),
                VersionSpec::parse("5.0.0").unwrap()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_requirement_to_highest_from_list() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            let mut spec = ToolSpec::parse("5").unwrap();

            let result = Resolver::new(&tool)
                .resolve_version(&mut spec, false)
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("5.10.15").unwrap());
            assert!(spec.is_resolved());
        }
    }

    mod resolve_version_candidate {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_aliases_remapped_by_plugin() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            for (alias, expected) in [
                ("stable", "5.0.0"),
                ("unstable", "6.0.0-rc.1"),
                ("legacy", "4.10.15"),
            ] {
                let result = Resolver::new(&tool)
                    .resolve_version_candidate(
                        &UnresolvedVersionSpec::parse(alias).unwrap(),
                        false,
                        false,
                    )
                    .await
                    .unwrap();

                assert_eq!(
                    result,
                    VersionSpec::parse(expected).unwrap(),
                    "alias: {alias}"
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_latest_alias_from_list() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            // The plugin resolver doesn't handle "latest",
            // so it falls through to the loaded version list
            let result = Resolver::new(&tool)
                .resolve_version_candidate(
                    &UnresolvedVersionSpec::parse("latest").unwrap(),
                    false,
                    false,
                )
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("5.10.15").unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn short_circuits_fully_qualified_without_validation() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            // This version doesn't exist in the version list,
            // but short circuiting trusts it as-is
            let result = Resolver::new(&tool)
                .resolve_version_candidate(
                    &UnresolvedVersionSpec::parse("999.999.999").unwrap(),
                    true,
                    false,
                )
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("999.999.999").unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn validates_fully_qualified_when_not_short_circuited() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            let result = Resolver::new(&tool)
                .resolve_version_candidate(
                    &UnresolvedVersionSpec::parse("999.999.999").unwrap(),
                    false,
                    false,
                )
                .await;

            assert!(result.is_err());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_canary_without_validation() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            // Canary short circuits even when short_circuit=false
            let result = Resolver::new(&tool)
                .resolve_version_candidate(&UnresolvedVersionSpec::Canary, false, false)
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::Canary);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_on_unknown_alias() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            let result = Resolver::new(&tool)
                .resolve_version_candidate(
                    &UnresolvedVersionSpec::parse("nonexistent").unwrap(),
                    false,
                    false,
                )
                .await;

            assert!(result.is_err());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_on_scoped_requirement_without_scoped_versions() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            // A scoped requirement only matches versions with the same
            // scope, and the mocked version list has none
            let result = Resolver::new(&tool)
                .resolve_version_candidate(
                    &UnresolvedVersionSpec::parse("temurin-5").unwrap(),
                    false,
                    false,
                )
                .await;

            assert!(result.is_err());
        }
    }

    mod resolve_version_from_list {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_from_loaded_versions() {
            let sandbox = create_empty_sandbox();
            let tool = create_node(sandbox.path()).await;

            let mut resolver = Resolver::new(&tool);
            let candidate = UnresolvedVersionSpec::parse("20").unwrap();
            resolver.load_versions(&candidate).await.unwrap();

            let result = resolver
                .resolve_version_from_list(&candidate, false)
                .await
                .unwrap();

            let resolved = result.to_string();
            assert!(
                resolved.starts_with("20."),
                "Expected 20.x.x, got {resolved}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_none_on_unknown_version() {
            let sandbox = create_empty_sandbox();
            let tool = create_node(sandbox.path()).await;

            let mut resolver = Resolver::new(&tool);
            // Load versions for a real range first
            let candidate = UnresolvedVersionSpec::parse("20").unwrap();
            resolver.load_versions(&candidate).await.unwrap();

            // Try to resolve a version that doesn't exist
            let bad_candidate = UnresolvedVersionSpec::parse("999.999.999").unwrap();
            let result = resolver
                .resolve_version_from_list(&bad_candidate, false)
                .await;

            assert!(result.is_none());
        }
    }

    mod load_versions {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn loads_remote_versions() {
            let sandbox = create_empty_sandbox();
            let tool = create_node(sandbox.path()).await;

            let mut resolver = Resolver::new(&tool);
            let initial = UnresolvedVersionSpec::parse("20").unwrap();

            resolver.load_versions(&initial).await.unwrap();

            // After loading, the data resolver should have versions
            let result = resolver.resolve_version_from_list(&initial, false).await;

            assert!(result.is_some());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn caches_versions_without_scope() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            Resolver::new(&tool)
                .load_versions(&UnresolvedVersionSpec::parse("5").unwrap())
                .await
                .unwrap();

            assert!(tool.inventory.dir.join("remote-versions.json").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn caches_versions_by_scope() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            Resolver::new(&tool)
                .load_versions(&UnresolvedVersionSpec::parse("temurin-5").unwrap())
                .await
                .unwrap();

            assert!(
                tool.inventory
                    .dir
                    .join("remote-versions-temurin.json")
                    .exists()
            );
            assert!(!tool.inventory.dir.join("remote-versions.json").exists());
        }
    }

    mod scoped_requirement_as_alias {
        use super::*;

        #[test]
        fn scoped_alias_names_parse_as_requirements() {
            // If these ever parse as aliases, the resolver work around
            // in `is_scoped_req_an_alias` is no longer necessary
            for value in ["next-12", "next-12.3"] {
                assert!(
                    matches!(
                        UnresolvedVersionSpec::parse(value).unwrap(),
                        UnresolvedVersionSpec::Requirement(_)
                    ),
                    "expected {value} to parse as a requirement"
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_scope_and_major_as_an_alias() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            // The version list has no "next" scoped versions, so this can only
            // resolve by treating "next-12" as an alias
            seed_remote_versions(
                &tool,
                "next",
                r#"{
                    "aliases": { "next-12": "1.2.3" },
                    "versions": ["1.0.0", "1.2.3", "2.0.0"]
                }"#,
            );

            let result = Resolver::new(&tool)
                .resolve_version_candidate(
                    &UnresolvedVersionSpec::parse("next-12").unwrap(),
                    false,
                    false,
                )
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("1.2.3").unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_scope_major_and_minor_as_an_alias() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            seed_remote_versions(
                &tool,
                "next",
                r#"{
                    "aliases": { "next-12.3": "1.2.3" },
                    "versions": ["1.0.0", "1.2.3", "2.0.0"]
                }"#,
            );

            let result = Resolver::new(&tool)
                .resolve_version_candidate(
                    &UnresolvedVersionSpec::parse("next-12.3").unwrap(),
                    false,
                    false,
                )
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("1.2.3").unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_an_alias_that_points_to_another_alias() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            seed_remote_versions(
                &tool,
                "next",
                r#"{
                    "aliases": { "next-12": "stable-ish", "stable-ish": "1.2.3" },
                    "versions": ["1.0.0", "1.2.3", "2.0.0"]
                }"#,
            );

            let result = Resolver::new(&tool)
                .resolve_version_candidate(
                    &UnresolvedVersionSpec::parse("next-12").unwrap(),
                    false,
                    false,
                )
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("1.2.3").unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_through_the_public_spec_entry_point() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            seed_remote_versions(
                &tool,
                "next",
                r#"{
                    "aliases": { "next-12": "1.2.3" },
                    "versions": ["1.0.0", "1.2.3", "2.0.0"]
                }"#,
            );

            let mut spec = ToolSpec::parse("next-12").unwrap();

            let result = Resolver::new(&tool)
                .resolve_version(&mut spec, false)
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("1.2.3").unwrap());
            assert!(spec.is_resolved());
            assert_eq!(
                spec.get_resolved_version(),
                VersionSpec::parse("1.2.3").unwrap()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn still_resolves_scoped_requirements_when_no_alias_matches() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            seed_remote_versions(
                &tool,
                "temurin",
                r#"{
                    "versions": ["temurin-5.0.0", "temurin-5.1.0", "temurin-6.0.0"]
                }"#,
            );

            let result = Resolver::new(&tool)
                .resolve_version_candidate(
                    &UnresolvedVersionSpec::parse("temurin-5").unwrap(),
                    false,
                    false,
                )
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("temurin-5.1.0").unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prefers_the_scoped_requirement_over_an_unrelated_alias() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            // The alias is for a different major, so it must not be applied
            seed_remote_versions(
                &tool,
                "temurin",
                r#"{
                    "aliases": { "temurin-6": "temurin-6.0.0" },
                    "versions": ["temurin-5.0.0", "temurin-5.1.0", "temurin-6.0.0"]
                }"#,
            );

            let result = Resolver::new(&tool)
                .resolve_version_candidate(
                    &UnresolvedVersionSpec::parse("temurin-5").unwrap(),
                    false,
                    false,
                )
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("temurin-5.1.0").unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_when_no_alias_and_no_scoped_versions_match() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            seed_remote_versions(
                &tool,
                "next",
                r#"{
                    "aliases": { "next-13": "1.2.3" },
                    "versions": ["1.0.0", "1.2.3", "2.0.0"]
                }"#,
            );

            let result = Resolver::new(&tool)
                .resolve_version_candidate(
                    &UnresolvedVersionSpec::parse("next-12").unwrap(),
                    false,
                    false,
                )
                .await;

            assert!(result.is_err());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_not_apply_to_unscoped_requirements() {
            let sandbox = create_empty_sandbox();
            let tool = create_mocked_tool(sandbox.path()).await;

            // An unscoped requirement has no scope to build an alias from,
            // so it resolves against the version list as usual
            let result = Resolver::new(&tool)
                .resolve_version_candidate(
                    &UnresolvedVersionSpec::parse("5").unwrap(),
                    false,
                    false,
                )
                .await
                .unwrap();

            assert_eq!(result, VersionSpec::parse("5.10.15").unwrap());
        }
    }
}
