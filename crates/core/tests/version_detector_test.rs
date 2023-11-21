use proto_core::{
    detect_version_first_available, detect_version_prefer_prototools, load_tool_from_locator,
    ProtoEnvironment, Tool, ToolsConfig, UnresolvedVersionSpec,
};
use starbase_sandbox::create_empty_sandbox;
use std::path::Path;
use warpgate::Id;

mod version_detector {
    use super::*;

    async fn create_node(_root: &Path) -> Tool {
        load_tool_from_locator(
            Id::raw("node"),
            ProtoEnvironment::new().unwrap(),
            ToolsConfig::builtin_plugins().get("node").unwrap(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn uses_deepest_prototools() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/.prototools", "node = \"20\"");
        sandbox.create_file("a/b/.prototools", "node = \"18\"");
        sandbox.create_file("a/b/c/.prototools", "node = \"16\"");

        let tool = create_node(sandbox.path()).await;

        assert_eq!(
            detect_version_first_available(&tool, &sandbox.path().join("a/b/c"), sandbox.path())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~16").unwrap())
        );

        assert_eq!(
            detect_version_first_available(&tool, &sandbox.path().join("a/b"), sandbox.path())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~18").unwrap())
        );

        assert_eq!(
            detect_version_first_available(&tool, &sandbox.path().join("a"), sandbox.path())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~20").unwrap())
        );
    }

    #[tokio::test]
    async fn finds_first_available_prototools() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/.prototools", "node = \"20\"");
        sandbox.create_file("package.json", r#"{ "engines": { "node": "18" } }"#);

        let tool = create_node(sandbox.path()).await;

        assert_eq!(
            detect_version_first_available(&tool, &sandbox.path().join("a/b"), sandbox.path())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~20").unwrap())
        );
    }

    #[tokio::test]
    async fn finds_first_available_ecosystem() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", "node = \"20\"");
        sandbox.create_file("a/package.json", r#"{ "engines": { "node": "18" } }"#);

        let tool = create_node(sandbox.path()).await;

        assert_eq!(
            detect_version_first_available(&tool, &sandbox.path().join("a/b"), sandbox.path())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~18").unwrap())
        );
    }

    #[tokio::test]
    async fn prefers_prototools() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/.prototools", "node = \"20\"");
        sandbox.create_file("a/b/.prototools", "node = \"18\"");
        sandbox.create_file("a/b/package.json", r#"{ "engines": { "node": "17" } }"#);
        sandbox.create_file("a/b/c/package.json", r#"{ "engines": { "node": "19" } }"#);

        let tool = create_node(sandbox.path()).await;

        assert_eq!(
            detect_version_prefer_prototools(&tool, &sandbox.path().join("a/b/c"), sandbox.path())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~18").unwrap())
        );
    }
}
