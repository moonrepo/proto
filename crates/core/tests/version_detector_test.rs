use proto_core::{
    ProtoConfig, ProtoEnvironment, ProtoFileManager, Tool, ToolContext, UnresolvedVersionSpec,
    flow::detect::{
        detect_version_first_available, detect_version_only_prototools,
        detect_version_prefer_prototools,
    },
    load_tool_from_locator,
};
use starbase_sandbox::create_empty_sandbox;
use std::path::Path;

mod version_detector {
    use super::*;

    async fn create_node(_root: &Path) -> Tool {
        load_tool_from_locator(
            ToolContext::parse("node").unwrap(),
            ProtoEnvironment::new().unwrap(),
            ProtoConfig::default()
                .builtin_plugins()
                .get("node")
                .unwrap(),
        )
        .await
        .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uses_deepest_prototools() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/.prototools", "node = \"20\"");
        sandbox.create_file("a/b/.prototools", "node = \"18\"");
        sandbox.create_file("a/b/c/.prototools", "node = \"16\"");

        let tool = create_node(sandbox.path()).await;
        let manager = ProtoFileManager::load(sandbox.path().join("a/b/c"), None, None).unwrap();

        assert_eq!(
            detect_version_first_available(&tool, &manager.get_config_files())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~16").unwrap())
        );

        let manager = ProtoFileManager::load(sandbox.path().join("a/b"), None, None).unwrap();

        assert_eq!(
            detect_version_first_available(&tool, &manager.get_config_files())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~18").unwrap())
        );

        let manager = ProtoFileManager::load(sandbox.path().join("a"), None, None).unwrap();

        assert_eq!(
            detect_version_first_available(&tool, &manager.get_config_files())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~20").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn finds_first_available_prototools() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/.prototools", "node = \"20\"");
        sandbox.create_file("package.json", r#"{ "engines": { "node": "18" } }"#);

        let tool = create_node(sandbox.path()).await;
        let manager = ProtoFileManager::load(sandbox.path().join("a/b"), None, None).unwrap();

        assert_eq!(
            detect_version_first_available(&tool, &manager.get_config_files())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~20").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn finds_first_available_ecosystem() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", "node = \"20\"");
        sandbox.create_file("a/package.json", r#"{ "engines": { "node": "18" } }"#);

        let tool = create_node(sandbox.path()).await;
        let manager = ProtoFileManager::load(sandbox.path().join("a/b"), None, None).unwrap();

        assert_eq!(
            detect_version_first_available(&tool, &manager.get_config_files())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~18").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn prefers_prototools() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/.prototools", "node = \"20\"");
        sandbox.create_file("a/b/.prototools", "node = \"18\"");
        sandbox.create_file("a/b/package.json", r#"{ "engines": { "node": "17" } }"#);
        sandbox.create_file("a/b/c/package.json", r#"{ "engines": { "node": "19" } }"#);

        let tool = create_node(sandbox.path()).await;
        let manager = ProtoFileManager::load(sandbox.path().join("a/b/c"), None, None).unwrap();

        assert_eq!(
            detect_version_prefer_prototools(&tool, &manager.get_config_files())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~18").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn only_uses_prototools() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/package.json", r#"{ "engines": { "node": "16" } }"#);
        sandbox.create_file("a/b/.prototools", "node = \"18\"");
        sandbox.create_file("a/b/package.json", r#"{ "engines": { "node": "17" } }"#);

        let tool = create_node(sandbox.path()).await;
        let manager = ProtoFileManager::load(sandbox.path().join("a/b"), None, None).unwrap();

        assert_eq!(
            detect_version_only_prototools(&tool, &manager.get_config_files())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~18").unwrap())
        );

        let manager = ProtoFileManager::load(sandbox.path().join("a"), None, None).unwrap();

        assert_eq!(
            detect_version_only_prototools(&tool, &manager.get_config_files())
                .await
                .unwrap(),
            None
        );
    }
}
