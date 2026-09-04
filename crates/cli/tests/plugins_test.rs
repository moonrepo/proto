use proto_core::flow::locate::Locator;
use proto_core::flow::manage::Manager;
use proto_core::test_utils::*;
use proto_core::{
    PluginLocator, ProtoEnvironment, ProtoLoaderError, Tool, ToolContext, ToolSpec,
    UnresolvedVersionSpec, flow::install::InstallOptions, load_tool_from_locator,
    warpgate::FileLocator, warpgate::UrlLocator,
};
use starbase_sandbox::predicates::prelude::*;
use std::env;
use std::fs;
use std::path::PathBuf;

fn create_empty_proto_sandbox_with_tools(ext: &str) -> ProtoSandbox {
    let sandbox = create_empty_proto_sandbox();
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("./tests/__fixtures__")
        .join(format!("moon-schema.{ext}"));

    sandbox.create_file(
        ".prototools",
        format!(
            r#"
moon-test = "1.0.0"

[plugins.tools]
moon-test = "file://{}"
"#,
            schema_path.to_string_lossy().replace("\\", "/")
        ),
    );

    sandbox
}

async fn run_tests<F, Fut>(factory: F)
where
    F: FnOnce(&ProtoEnvironment) -> Fut,
    Fut: Future<Output = Result<Tool, ProtoLoaderError>>,
{
    let sandbox = create_empty_proto_sandbox();
    let proto = ProtoEnvironment::new_testing(sandbox.path()).unwrap();

    // Paths must exist for things to work correctly!
    fs::create_dir_all(&proto.store.dir).unwrap();
    fs::create_dir_all(&proto.home_dir).unwrap();

    let mut tool = factory(&proto).await.unwrap();
    let mut spec = ToolSpec::new(UnresolvedVersionSpec::parse("1.0.0").unwrap());

    Manager::new(&mut tool)
        .install(&mut spec, InstallOptions::default())
        .await
        .unwrap();

    assert!(tool.get_product_dir(&spec).exists());

    let base_dir = proto.store.inventory_dir.join("moon/1.0.0");

    let mut locator = Locator::new(&tool, &spec);

    if cfg!(windows) {
        assert_eq!(
            locator.locate_exe_file().await.unwrap(),
            base_dir.join("moon.exe")
        );
        assert!(proto.store.shims_dir.join("moon.exe").exists());
    } else {
        assert_eq!(
            locator.locate_exe_file().await.unwrap(),
            base_dir.join("moon")
        );
        assert!(proto.store.shims_dir.join("moon").exists());
    }
}

mod plugins {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_and_installs_toml_plugin_from_file() {
        run_tests(|env| {
            let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

            load_tool_from_locator(
                ToolContext::parse("moon").unwrap(),
                env.to_owned(),
                PluginLocator::File(Box::new(FileLocator {
                    file: "./tests/__fixtures__/moon-schema.toml".into(),
                    path: Some(root_dir.join("./tests/__fixtures__/moon-schema.toml")),
                })),
            )
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_and_installs_json_plugin_from_file() {
        run_tests(|env| {
            let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

            load_tool_from_locator(
                ToolContext::parse("moon").unwrap(),
                env.to_owned(),
                PluginLocator::File(Box::new(FileLocator {
                    file: "./tests/__fixtures__/moon-schema.json".into(),
                    path: Some(root_dir.join("./tests/__fixtures__/moon-schema.json")),
                })),
            )
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_and_installs_yaml_plugin_from_file() {
        run_tests(|env| {
            let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

            load_tool_from_locator(
                ToolContext::parse("moon").unwrap(),
                env.to_owned(),
                PluginLocator::File(Box::new(FileLocator {
                    file: "./tests/__fixtures__/moon-schema.yaml".into(),
                    path: Some(root_dir.join("./tests/__fixtures__/moon-schema.yaml")),
                })),
            )
        })
        .await;
    }

    #[tokio::test]
    #[should_panic(expected = "MissingSourceFile")]
    async fn errors_for_missing_file() {
        run_tests(|env| {
            let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

            load_tool_from_locator(
                ToolContext::parse("moon").unwrap(),
                env.to_owned(),
                PluginLocator::File(Box::new(FileLocator {
                    file: "./some/fake/path.toml".into(),
                    path: Some(root_dir.join("./some/fake/path.toml")),
                })),
            )
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_and_installs_plugin_from_url() {
        run_tests(|env| {
            load_tool_from_locator(
                ToolContext::parse("moon").unwrap(),
                env.to_owned(),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://raw.githubusercontent.com/moonrepo/proto/refs/heads/master/crates/cli/tests/__fixtures__/moon-schema.toml"
                        .into(),
                })),
            )
        })
        .await;
    }

    #[tokio::test]
    #[should_panic(expected = "NotFound")]
    async fn errors_for_broken_url() {
        run_tests(|env| {
            load_tool_from_locator(
                ToolContext::parse("moon").unwrap(),
                env.to_owned(),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://raw.githubusercontent.com/moonrepo/moon/some/fake/path.toml"
                        .into(),
                })),
            )
        })
        .await;
    }

    mod builtins {
        use super::*;

        #[test]
        fn supports_toml_schema() {
            let sandbox = create_empty_proto_sandbox_with_tools("toml");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("moon-test");
                })
                .success();

            // Doesn't create shims
        }

        #[test]
        fn supports_json_schema() {
            let sandbox = create_empty_proto_sandbox_with_tools("json");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("moon-test");
                })
                .success();

            // Doesn't create shims
        }

        #[test]
        fn supports_yaml_schema() {
            let sandbox = create_empty_proto_sandbox_with_tools("yaml");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("moon-test");
                })
                .success();

            // Doesn't create shims
        }

        #[test]
        fn errors_if_disabled() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
[settings]
builtin-plugins = false
"#,
            );

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("go");
                })
                .failure();

            assert.stderr(predicate::str::contains("Unable to proceed, go"));
        }
    }
}
