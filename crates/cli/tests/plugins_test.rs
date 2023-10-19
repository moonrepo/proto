mod utils;

use futures::Future;
use proto_core::{
    load_tool_from_locator, Id, PluginLocator, ProtoEnvironment, Tool, UnresolvedVersionSpec,
    UserConfig,
};
use std::env;
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
        assert_eq!(tool.get_bin_path().unwrap(), &base_dir.join("moon.exe"));
        assert!(proto.shims_dir.join("moon.cmd").exists());
    } else {
        assert_eq!(tool.get_bin_path().unwrap(), &base_dir.join("moon"));
        assert!(proto.shims_dir.join("moon").exists());
    }
}

#[tokio::test]
async fn downloads_and_installs_plugin_from_file() {
    let user_config = UserConfig::default();

    run_tests(|root| {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        load_tool_from_locator(
            Id::raw("moon"),
            ProtoEnvironment::from(root).unwrap(),
            PluginLocator::SourceFile {
                file: "./tests/fixtures/moon-schema.toml".into(),
                path: root_dir.join("./tests/fixtures/moon-schema.toml"),
            },
            &user_config,
        )
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "does not exist")]
async fn errors_for_missing_file() {
    let user_config = UserConfig::default();

    run_tests(|root| {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        load_tool_from_locator(
            Id::raw("moon"),
            ProtoEnvironment::from(root).unwrap(),
            PluginLocator::SourceFile {
                file: "./some/fake/path.toml".into(),
                path: root_dir.join("./some/fake/path.toml"),
            },
            &user_config,
        )
    })
    .await;
}

#[tokio::test]
async fn downloads_and_installs_plugin_from_url() {
    let user_config = UserConfig::default();

    run_tests(|root| {
        load_tool_from_locator(
            Id::raw("moon"),
            ProtoEnvironment::from(root).unwrap(),
            PluginLocator::SourceUrl {
                url: "https://raw.githubusercontent.com/moonrepo/moon/master/proto-plugin.toml"
                    .into(),
            },
            &user_config,
        )
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "does not exist")]
async fn errors_for_broken_url() {
    let user_config = UserConfig::default();

    run_tests(|root| {
        load_tool_from_locator(
            Id::raw("moon"),
            ProtoEnvironment::from(root).unwrap(),
            PluginLocator::SourceUrl {
                url: "https://raw.githubusercontent.com/moonrepo/moon/some/fake/path.toml".into(),
            },
            &user_config,
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
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("bun").assert();

        assert.success();
    }

    #[test]
    fn supports_deno() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("deno").assert();

        assert.success();
    }

    #[test]
    fn supports_go() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("go").assert();

        assert.success();
    }

    #[test]
    fn supports_node() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("install")
            .arg("node")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        assert.success();
    }

    #[test]
    fn supports_npm() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("npm").assert();

        assert.success();
    }

    #[test]
    fn supports_pnpm() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("pnpm").assert();

        assert.success();
    }

    #[test]
    fn supports_yarn() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("yarn").assert();

        assert.success();
    }

    #[test]
    fn supports_python() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("python").assert();

        assert.success();
    }

    #[test]
    fn supports_rust() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("rust").assert();

        assert.success();
    }

    #[test]
    fn supports_toml_schema() {
        let temp = create_empty_sandbox_with_tools();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("moon-test").assert();

        assert.success();
    }
}
