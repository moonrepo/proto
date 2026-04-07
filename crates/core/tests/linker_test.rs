use proto_core::{
    ProtoConfig, ProtoEnvironment, Tool, ToolContext, ToolSpec, flow::link::Linker,
    load_tool_from_locator,
};
use starbase_sandbox::create_empty_sandbox;
use std::path::Path;
use version_spec::VersionSpec;

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

mod linker {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn link_bins_returns_empty_when_no_installed_version() {
        let sandbox = create_empty_sandbox();
        let tool = create_node(sandbox.path()).await;

        // Use a version that is not installed
        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let linker = Linker::new(&tool, &spec);

        // link_bins should not error, but bins may be empty since nothing is installed
        let bins = linker.link_bins(false).await.unwrap();

        // Since 20.0.0 is not installed, no source files exist to symlink
        // So either empty or all skipped due to missing source
        assert!(
            bins.is_empty(),
            "Should return no bins when version not installed"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn link_shims_creates_files() {
        let sandbox = create_empty_sandbox();
        let mut proto = ProtoEnvironment::new_testing(sandbox.path()).unwrap();
        proto.working_dir = sandbox.path().to_path_buf();

        let tool = load_tool_from_locator(
            ToolContext::parse("node").unwrap(),
            proto,
            ProtoConfig::default()
                .builtin_plugins()
                .tools
                .get("node")
                .unwrap(),
        )
        .await
        .unwrap();

        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let linker = Linker::new(&tool, &spec);

        // Force create shims
        let shims = linker.link_shims(true).await.unwrap();

        if !shims.is_empty() {
            // Verify shim files exist on disk
            for shim_path in &shims {
                assert!(
                    shim_path.exists(),
                    "Shim file should exist: {}",
                    shim_path.display()
                );
            }

            // Verify they are in the shims directory
            let shims_dir = tool.proto.store.shims_dir.clone();
            for shim_path in &shims {
                assert!(
                    shim_path.starts_with(&shims_dir),
                    "Shim should be in shims_dir: {}",
                    shim_path.display()
                );
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn link_all_returns_both_bins_and_shims() {
        let sandbox = create_empty_sandbox();
        let mut proto = ProtoEnvironment::new_testing(sandbox.path()).unwrap();
        proto.working_dir = sandbox.path().to_path_buf();

        let tool = load_tool_from_locator(
            ToolContext::parse("node").unwrap(),
            proto,
            ProtoConfig::default()
                .builtin_plugins()
                .tools
                .get("node")
                .unwrap(),
        )
        .await
        .unwrap();

        let spec = ToolSpec::new_resolved(VersionSpec::parse("20.0.0").unwrap());
        let response = Linker::link(&tool, &spec, true).await.unwrap();

        // Response should have shims (bins may be empty without installation)
        // The response itself should always be a valid struct
        assert!(
            response.bins.is_empty(),
            "Bins should be empty without installed version"
        );
        // Shims should be created even without installation
        assert!(
            !response.shims.is_empty(),
            "Shims should be created for node"
        );
    }
}
