use proto_core::test_utils::create_empty_proto_sandbox;
use proto_core::{
    ProtoConfig, ProtoEnvironment, Tool, ToolContext, ToolSpec, flow::locate::Locator,
    load_tool_from_locator,
};
use starbase_utils::fs;
use std::path::Path;
use version_spec::VersionSpec;

async fn create_node(root: &Path) -> Tool {
    load_tool_from_locator(
        ToolContext::parse("node").unwrap(),
        ProtoEnvironment::new_testing(root).unwrap(),
        ProtoConfig::default()
            .builtin_plugins()
            .tools
            .get("node")
            .unwrap(),
    )
    .await
    .unwrap()
}

fn create_fake_exes(root: &Path) {
    fs::write_file(
        root.join(".proto/tools/node/20.0.0")
            .join(if cfg!(windows) {
                "node.exe"
            } else {
                "bin/node"
            }),
        "",
    )
    .unwrap();
}

mod locator {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn product_dir_contains_version() {
        let sandbox = create_empty_proto_sandbox();
        let tool = create_node(sandbox.path()).await;

        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let locator = Locator::new(&tool, &spec);

        let product_dir = locator.product_dir.to_string_lossy().to_string();

        assert!(product_dir.contains("20.0.0"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn product_dir_contains_tool_id() {
        let sandbox = create_empty_proto_sandbox();
        let tool = create_node(sandbox.path()).await;

        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let locator = Locator::new(&tool, &spec);

        let product_dir = locator.product_dir.to_string_lossy().to_string();

        assert!(product_dir.contains("node"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_primary_exe() {
        let sandbox = create_empty_proto_sandbox();
        let tool = create_node(sandbox.path()).await;

        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let locator = Locator::new(&tool, &spec);

        let primary = locator.locate_primary_exe().await.unwrap();

        assert!(primary.is_some());

        let exe_loc = primary.unwrap();

        assert!(exe_loc.name == "node");
        assert!(exe_loc.config.primary);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_secondary_exes_without_error() {
        let sandbox = create_empty_proto_sandbox();
        let tool = create_node(sandbox.path()).await;

        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let locator = Locator::new(&tool, &spec);

        // Should not error; the result may be empty if the plugin
        // does not register secondary executables via LocateExecutables
        let secondaries = locator.locate_secondary_exes().await.unwrap();

        // All returned executables should not be primary
        for exe in &secondaries {
            assert!(!exe.config.primary);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_shim_paths() {
        let sandbox = create_empty_proto_sandbox();
        let tool = create_node(sandbox.path()).await;

        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let locator = Locator::new(&tool, &spec);

        let shims = locator.locate_shims().await.unwrap();

        assert!(!shims.is_empty());

        // Primary shim should be named "node"
        let has_node_shim = shims.iter().any(|s| s.name == "node");
        assert!(has_node_shim);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locate_exe_file_returns_path() {
        let sandbox = create_empty_proto_sandbox();
        let tool = create_node(sandbox.path()).await;

        create_fake_exes(sandbox.path());

        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let mut locator = Locator::new(&tool, &spec);

        let exe_file = locator.locate_exe_file().await.unwrap();

        let file_name = exe_file.file_name().unwrap().to_string_lossy().to_string();

        assert!(file_name.contains("node"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locate_exes_dirs_returns_dirs() {
        let sandbox = create_empty_proto_sandbox();
        let tool = create_node(sandbox.path()).await;

        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let mut locator = Locator::new(&tool, &spec);

        let dirs = locator.locate_exes_dirs().await.unwrap();

        assert!(!dirs.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locate_all_returns_complete_response() {
        let sandbox = create_empty_proto_sandbox();
        let tool = create_node(sandbox.path()).await;

        create_fake_exes(sandbox.path());

        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let mut locator = Locator::new(&tool, &spec);

        let response = locator.locate_all().await.unwrap();

        // Should have at minimum an exe_file and exes_dirs
        let file_name = response
            .exe_file
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(file_name.contains("node"));
        assert!(!response.exes_dirs.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn getters_return_none_before_locate() {
        let sandbox = create_empty_proto_sandbox();
        let tool = create_node(sandbox.path()).await;

        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let locator = Locator::new(&tool, &spec);

        // Before calling locate methods, getters should return None/empty
        assert!(locator.get_exe_file().is_none());
        assert!(locator.get_exes_dir().is_none());
        assert!(locator.get_exes_dirs().is_empty());
        assert!(locator.get_globals_dir().is_none());
        assert!(locator.get_globals_dirs().is_empty());
        assert!(locator.get_globals_prefix().is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn getters_populated_after_locate() {
        let sandbox = create_empty_proto_sandbox();
        let tool = create_node(sandbox.path()).await;

        create_fake_exes(sandbox.path());

        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let mut locator = Locator::new(&tool, &spec);

        locator.locate_all().await.unwrap();

        // After locate, exe_file and exes_dir should be populated
        assert!(locator.get_exe_file().is_some());
        assert!(!locator.get_exes_dirs().is_empty());
    }
}
