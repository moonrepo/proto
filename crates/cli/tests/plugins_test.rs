mod utils;

use proto_core::{
    PluginLocator, ProtoEnvironment, ProtoLoaderError, Tool, ToolContext, ToolSpec,
    UnresolvedVersionSpec, flow::install::InstallOptions, load_tool_from_locator,
    warpgate::FileLocator, warpgate::UrlLocator,
};
use starbase_sandbox::assert_snapshot;
use starbase_sandbox::predicates::prelude::*;
use std::env;
use std::fs;
use std::path::PathBuf;
use utils::*;

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

    tool.setup(
        &ToolSpec::new(UnresolvedVersionSpec::parse("1.0.0").unwrap()),
        InstallOptions::default(),
    )
    .await
    .unwrap();

    assert!(tool.get_product_dir().exists());

    let base_dir = proto.store.inventory_dir.join("moon/1.0.0");

    if cfg!(windows) {
        assert_eq!(
            &tool.locate_exe_file().await.unwrap(),
            &base_dir.join("moon.exe")
        );
        assert!(proto.store.shims_dir.join("moon.exe").exists());
    } else {
        assert_eq!(
            &tool.locate_exe_file().await.unwrap(),
            &base_dir.join("moon")
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
                    url: "https://raw.githubusercontent.com/moonrepo/moon/master/proto-plugin.toml"
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
        fn supports_bun() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("bun");
                })
                .success();

            create_shim_command(sandbox.path(), "bun")
                .arg("--version")
                .assert()
                .success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[test]
        fn supports_deno() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("deno");
                })
                .success();

            create_shim_command(sandbox.path(), "deno")
                .arg("--version")
                .assert()
                .success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[test]
        fn supports_go() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("go");
                })
                .success();

            create_shim_command(sandbox.path(), "go")
                .arg("version")
                .assert()
                .success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[test]
        fn supports_moon() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("moon");
                })
                .success();

            create_shim_command(sandbox.path(), "moon")
                .arg("--version")
                .assert()
                .success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[test]
        fn supports_node_and_package_managers() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("node");
                })
                .success();

            create_shim_command(sandbox.path(), "node")
                .arg("--version")
                .assert()
                .success();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("npm");
                })
                .success();

            create_shim_command(sandbox.path(), "npm")
                .arg("--version")
                .assert()
                .success();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("pnpm");
                })
                .success();

            create_shim_command(sandbox.path(), "pnpm")
                .arg("--version")
                .assert()
                .success();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("yarn");
                })
                .success();

            create_shim_command(sandbox.path(), "yarn")
                .arg("--version")
                .assert()
                .success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[test]
        fn supports_python() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("python").arg("3.12.0"); // Latest doesn't always work
                })
                .success();

            create_shim_command(sandbox.path(), "python")
                .arg("--version")
                .env("PROTO_PYTHON_VERSION", "3.12.0")
                .assert()
                .success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[test]
        fn supports_poetry() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("python").arg("3.12.0");
                })
                .success();

            // `poetry` is called in a post-install hook,
            // so we need to make it available on PATH
            let mut paths = vec![sandbox.path().join(".proto/shims")];
            paths.extend(starbase_utils::env::paths());

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("poetry")
                        .env("PATH", std::env::join_paths(paths).unwrap());
                })
                .success();

            create_shim_command(sandbox.path(), "poetry")
                .arg("--version")
                .assert()
                .success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[test]
        fn supports_uv() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("uv");
                })
                .success();

            create_shim_command(sandbox.path(), "uv")
                .arg("--version")
                .assert()
                .success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[cfg(unix)]
        #[test]
        fn supports_ruby() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("ruby").arg("--yes");
                })
                .success();

            create_shim_command(sandbox.path(), "ruby")
                .arg("--version")
                .assert()
                .success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[test]
        fn supports_rust() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("rust");
                })
                .success();

            // Doesn't create shims
        }

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
