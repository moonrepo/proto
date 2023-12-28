mod utils;

use proto_core::{
    load_tool_from_locator, Id, PluginLocator, ProtoEnvironment, Tool, UnresolvedVersionSpec,
};
use starbase_sandbox::assert_snapshot;
use std::env;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use utils::*;

async fn run_tests<F, Fut>(factory: F)
where
    F: FnOnce(&Path) -> Fut,
    Fut: Future<Output = miette::Result<Tool>>,
{
    let fixture = create_empty_sandbox();
    let proto = ProtoEnvironment::from(fixture.path()).unwrap();

    let mut tool = factory(fixture.path()).await.unwrap();

    env::set_var("PROTO_HOME", fixture.path().to_string_lossy().to_string());

    tool.setup(&UnresolvedVersionSpec::parse("1.0.0").unwrap(), false)
        .await
        .unwrap();

    env::remove_var("PROTO_HOME");

    assert!(tool.get_tool_dir().exists());

    let base_dir = proto.tools_dir.join("moon/1.0.0");

    if cfg!(windows) {
        assert_eq!(tool.get_exe_path().unwrap(), &base_dir.join("moon.exe"));
        assert!(proto.shims_dir.join("moon.exe").exists());
    } else {
        assert_eq!(tool.get_exe_path().unwrap(), &base_dir.join("moon"));
        assert!(proto.shims_dir.join("moon").exists());
    }
}

mod plugins {
    use super::*;

    #[tokio::test]
    async fn downloads_and_installs_plugin_from_file() {
        run_tests(|root| {
            let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

            load_tool_from_locator(
                Id::raw("moon"),
                ProtoEnvironment::from(root).unwrap(),
                PluginLocator::SourceFile {
                    file: "./tests/fixtures/moon-schema.toml".into(),
                    path: root_dir.join("./tests/fixtures/moon-schema.toml"),
                },
            )
        })
        .await;
    }

    #[tokio::test]
    #[should_panic(expected = "does not exist")]
    async fn errors_for_missing_file() {
        run_tests(|root| {
            let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

            load_tool_from_locator(
                Id::raw("moon"),
                ProtoEnvironment::from(root).unwrap(),
                PluginLocator::SourceFile {
                    file: "./some/fake/path.toml".into(),
                    path: root_dir.join("./some/fake/path.toml"),
                },
            )
        })
        .await;
    }

    #[tokio::test]
    async fn downloads_and_installs_plugin_from_url() {
        run_tests(|root| {
            load_tool_from_locator(
                Id::raw("moon"),
                ProtoEnvironment::from(root).unwrap(),
                PluginLocator::SourceUrl {
                    url: "https://raw.githubusercontent.com/moonrepo/moon/master/proto-plugin.toml"
                        .into(),
                },
            )
        })
        .await;
    }

    #[tokio::test]
    #[should_panic(expected = "does not exist")]
    async fn errors_for_broken_url() {
        run_tests(|root| {
            load_tool_from_locator(
                Id::raw("moon"),
                ProtoEnvironment::from(root).unwrap(),
                PluginLocator::SourceUrl {
                    url: "https://raw.githubusercontent.com/moonrepo/moon/some/fake/path.toml"
                        .into(),
                },
            )
        })
        .await;
    }

    mod builtins {
        use super::*;

        // Bun doesn't support Windows
        #[cfg(not(windows))]
        #[test]
        fn supports_bun() {
            use starbase_sandbox::get_assert_output;

            let sandbox = create_empty_sandbox();

            let assert = create_proto_command(sandbox.path())
                .arg("install")
                .arg("bun")
                .assert();

            println!("{}", get_assert_output(&assert));

            assert.success();

            // Try and get output with spawn
            println!("------ SPAWN START ------");
            create_shim_command_std(sandbox.path(), "bun")
                .arg("--version")
                .spawn()
                .unwrap();
            println!("------ SPAWN END ------");

            let assert = create_shim_command(sandbox.path(), "bun")
                .arg("--version")
                .assert();

            println!("{}", get_assert_output(&assert));

            assert.success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[test]
        fn supports_deno() {
            let sandbox = create_empty_sandbox();

            create_proto_command(sandbox.path())
                .arg("install")
                .arg("deno")
                .assert()
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
            let sandbox = create_empty_sandbox();

            create_proto_command(sandbox.path())
                .arg("install")
                .arg("go")
                .assert()
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
        fn supports_node() {
            let sandbox = create_empty_sandbox();

            create_proto_command(sandbox.path())
                .arg("install")
                .arg("node")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert()
                .success();

            create_shim_command(sandbox.path(), "node")
                .arg("--version")
                .assert()
                .success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[test]
        fn supports_npm() {
            let sandbox = create_empty_sandbox();

            create_proto_command(sandbox.path())
                .arg("install")
                .arg("node")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert()
                .success();

            create_proto_command(sandbox.path())
                .arg("install")
                .arg("npm")
                .assert()
                .success();

            create_shim_command(sandbox.path(), "npm")
                .arg("--version")
                .assert()
                .success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[test]
        fn supports_pnpm() {
            let sandbox = create_empty_sandbox();

            create_proto_command(sandbox.path())
                .arg("install")
                .arg("node")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert()
                .success();

            create_proto_command(sandbox.path())
                .arg("install")
                .arg("pnpm")
                .assert()
                .success();

            create_shim_command(sandbox.path(), "pnpm")
                .arg("--version")
                .assert()
                .success();

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".proto/shims/registry.json")).unwrap()
            );
        }

        #[test]
        fn supports_yarn() {
            let sandbox = create_empty_sandbox();

            create_proto_command(sandbox.path())
                .arg("install")
                .arg("node")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert()
                .success();

            create_proto_command(sandbox.path())
                .arg("install")
                .arg("yarn")
                .assert()
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
            let sandbox = create_empty_sandbox();

            create_proto_command(sandbox.path())
                .arg("install")
                .arg("python")
                .arg("3.12.0") // Latest doesn't always work
                .assert()
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
        fn supports_rust() {
            let sandbox = create_empty_sandbox();

            create_proto_command(sandbox.path())
                .arg("install")
                .arg("rust")
                .assert();

            // Doesn't create shims
        }

        #[test]
        fn supports_toml_schema() {
            let sandbox = create_empty_sandbox_with_tools();

            create_proto_command(sandbox.path())
                .arg("install")
                .arg("moon-test")
                .assert()
                .success();

            // Doesn't create shims
        }
    }
}
