use proto_core::{
    ProtoConfig, ProtoEnvironment, ProtoFileManager, Tool, ToolContext, UnresolvedVersionSpec,
    flow::detect::Detector, load_tool_from_locator,
};
use starbase_sandbox::create_empty_sandbox;
use std::path::Path;

mod version_detector {
    use super::*;

    async fn create_node(root: &Path) -> Tool {
        let mut env = ProtoEnvironment::new().unwrap();

        // Map the sandbox so that the plugin can access version files
        env.working_dir = root.to_path_buf();

        load_tool_from_locator(
            ToolContext::parse("node").unwrap(),
            env,
            ProtoConfig::default()
                .builtin_plugins()
                .tools
                .get("node")
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn load_mocked_tool(env: ProtoEnvironment) -> Tool {
        load_tool_from_locator(
            ToolContext::parse("protostar").unwrap(),
            env,
            ProtoConfig::default()
                .builtin_plugins()
                .tools
                .get("protostar")
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn create_mocked_tool(root: &Path) -> Tool {
        let mut env = ProtoEnvironment::new_testing(root).unwrap();

        // Map the sandbox so that the plugin can access version files
        env.working_dir = root.to_path_buf();

        load_mocked_tool(env).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uses_deepest_prototools() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/.prototools", "node = \"20\"");
        sandbox.create_file("a/b/.prototools", "node = \"18\"");
        sandbox.create_file("a/b/c/.prototools", "node = \"16\"");

        let tool = create_node(sandbox.path()).await;
        let mut detector = Detector::new(&tool);
        let manager = ProtoFileManager::load(sandbox.path().join("a/b/c"), None, None).unwrap();

        assert_eq!(
            detector
                .detect_version_first_available(&manager.get_config_files())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~16").unwrap())
        );

        let manager = ProtoFileManager::load(sandbox.path().join("a/b"), None, None).unwrap();

        assert_eq!(
            detector
                .detect_version_first_available(&manager.get_config_files())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~18").unwrap())
        );

        let manager = ProtoFileManager::load(sandbox.path().join("a"), None, None).unwrap();

        assert_eq!(
            detector
                .detect_version_first_available(&manager.get_config_files())
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
            Detector::new(&tool)
                .detect_version_first_available(&manager.get_config_files())
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
            Detector::new(&tool)
                .detect_version_first_available(&manager.get_config_files())
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
            Detector::new(&tool)
                .detect_version_prefer_prototools(&manager.get_config_files())
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
            Detector::new(&tool)
                .detect_version_only_prototools(&manager.get_config_files())
                .await
                .unwrap(),
            Some(UnresolvedVersionSpec::parse("~18").unwrap())
        );

        let manager = ProtoFileManager::load(sandbox.path().join("a"), None, None).unwrap();

        assert_eq!(
            Detector::new(&tool)
                .detect_version_only_prototools(&manager.get_config_files())
                .await
                .unwrap(),
            None
        );
    }

    mod stop {
        use super::*;

        fn spec(version: &str) -> UnresolvedVersionSpec {
            UnresolvedVersionSpec::parse(version).unwrap()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn checks_next_file_without_stop() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".protostar.after-stop", "1.2.3");

            let tool = create_mocked_tool(sandbox.path()).await;

            assert_eq!(
                Detector::new(&tool)
                    .detect_version_from(sandbox.path())
                    .await
                    .unwrap(),
                Some((spec("1.2.3"), sandbox.path().join(".protostar.after-stop")))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_remaining_files_in_directory() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".protostar.stop", "");
            sandbox.create_file(".protostar.after-stop", "1.2.3");

            let tool = create_mocked_tool(sandbox.path()).await;

            assert_eq!(
                Detector::new(&tool)
                    .detect_version_from(sandbox.path())
                    .await
                    .unwrap(),
                None
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_version_from_stop_file() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".protostar.stop", "3.0.0");
            sandbox.create_file(".protostar.after-stop", "1.2.3");

            let tool = create_mocked_tool(sandbox.path()).await;

            assert_eq!(
                Detector::new(&tool)
                    .detect_version_from(sandbox.path())
                    .await
                    .unwrap(),
                Some((spec("3.0.0"), sandbox.path().join(".protostar.stop")))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn still_checks_parent_directories() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".protostar.after-stop", "2.0.0");
            sandbox.create_file("a/.protostar.stop", "");
            sandbox.create_file("a/.protostar.after-stop", "1.2.3");

            let tool = create_mocked_tool(sandbox.path()).await;
            let manager = ProtoFileManager::load(sandbox.path().join("a"), None, None).unwrap();
            let mut detector = Detector::new(&tool);

            assert_eq!(
                detector
                    .detect_version_first_available(&manager.get_config_files())
                    .await
                    .unwrap(),
                Some(spec("2.0.0"))
            );
            assert_eq!(
                detector.source,
                Some(sandbox.path().join(".protostar.after-stop"))
            );
        }
    }

    // Only the home directory, store, and working directory are mapped
    // into the plugin's filesystem, and the sandbox root is none of them
    mod unmapped_paths {
        use super::*;

        fn create_env(root: &Path, working_dir: &str) -> ProtoEnvironment {
            let mut env = ProtoEnvironment::new_testing(root).unwrap();
            env.working_dir = root.join(working_dir);
            env
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_parent_of_working_dir() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".protostarrc", "1.2.3");
            sandbox.create_file("a/.gitkeep", "");

            let tool = load_mocked_tool(create_env(sandbox.path(), "a")).await;
            let manager = ProtoFileManager::load(sandbox.path().join("a"), None, None).unwrap();

            assert!(
                !tool
                    .plugin
                    .virtual_paths
                    .iter()
                    .any(|(host, _)| sandbox.path().starts_with(host))
            );

            assert_eq!(
                Detector::new(&tool)
                    .detect_version_first_available(&manager.get_config_files())
                    .await
                    .unwrap(),
                None
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn reads_working_dir() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a/.protostarrc", "1.2.3");

            let tool = load_mocked_tool(create_env(sandbox.path(), "a")).await;
            let dir = sandbox.path().join("a");

            assert_eq!(
                Detector::new(&tool)
                    .detect_version_from(&dir)
                    .await
                    .unwrap(),
                Some((
                    UnresolvedVersionSpec::parse("1.2.3").unwrap(),
                    dir.join(".protostarrc")
                ))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn reads_file_within_home_dir() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".home/project/.protostarrc", "1.2.3");

            let tool = load_mocked_tool(create_env(sandbox.path(), "a")).await;
            let dir = sandbox.path().join(".home/project");

            assert_eq!(
                Detector::new(&tool)
                    .detect_version_from(&dir)
                    .await
                    .unwrap(),
                Some((
                    UnresolvedVersionSpec::parse("1.2.3").unwrap(),
                    dir.join(".protostarrc")
                ))
            );
        }
    }
}
