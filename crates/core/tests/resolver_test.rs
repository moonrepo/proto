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
    }

    mod resolve_version_candidate {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_from_loaded_versions() {
            let sandbox = create_empty_sandbox();
            let tool = create_node(sandbox.path()).await;

            let mut resolver = Resolver::new(&tool);
            let candidate = UnresolvedVersionSpec::parse("20").unwrap();
            resolver.load_versions(&candidate).await.unwrap();

            let result = resolver
                .resolve_version_candidate(&candidate, false)
                .await
                .unwrap();

            let resolved = result.to_string();
            assert!(
                resolved.starts_with("20."),
                "Expected 20.x.x, got {resolved}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_on_unknown_version() {
            let sandbox = create_empty_sandbox();
            let tool = create_node(sandbox.path()).await;

            let mut resolver = Resolver::new(&tool);
            // Load versions for a real range first
            let candidate = UnresolvedVersionSpec::parse("20").unwrap();
            resolver.load_versions(&candidate).await.unwrap();

            // Try to resolve a version that doesn't exist
            let bad_candidate = UnresolvedVersionSpec::parse("999.999.999").unwrap();
            let result = resolver
                .resolve_version_candidate(&bad_candidate, false)
                .await;

            assert!(result.is_err());
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
            let result = resolver.resolve_version_candidate(&initial, false).await;

            assert!(result.is_ok());
        }
    }
}
