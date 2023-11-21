use proto_core::{DetectStrategy, PinType, UserConfig, USER_CONFIG_NAME};
use starbase_sandbox::create_empty_sandbox;
use std::collections::BTreeMap;
use std::env;
use warpgate::{GitHubLocator, HttpOptions, Id, PluginLocator};

mod user_config {
    use super::*;

    #[test]
    fn loads_defaults_if_missing() {
        let sandbox = create_empty_sandbox();
        let config = UserConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(
            config,
            UserConfig {
                auto_clean: false,
                auto_install: false,
                detect_strategy: DetectStrategy::default(),
                node_intercept_globals: true,
                http: HttpOptions::default(),
                pin_latest: None,
                plugins: BTreeMap::default(),
                tools: BTreeMap::default(),
                path: sandbox.path().join(USER_CONFIG_NAME),
            }
        );
    }

    #[test]
    fn can_set_values() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            "config.toml",
            r#"
auto-clean = true
auto-install = true
node-intercept-globals = false
pin-latest = "global"
"#,
        );

        let config = UserConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(
            config,
            UserConfig {
                auto_clean: true,
                auto_install: true,
                detect_strategy: DetectStrategy::default(),
                node_intercept_globals: false,
                http: HttpOptions::default(),
                pin_latest: Some(PinType::Global),
                plugins: BTreeMap::default(),
                tools: BTreeMap::default(),
                path: sandbox.path().join(USER_CONFIG_NAME),
            }
        );
    }

    #[test]
    fn can_set_booleans_from_env_vars() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("config.toml", "");

        env::set_var("PROTO_AUTO_CLEAN", "1");
        env::set_var("PROTO_AUTO_INSTALL", "true");
        env::set_var("PROTO_NODE_INTERCEPT_GLOBALS", "off");

        let config = UserConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(
            config,
            UserConfig {
                auto_clean: true,
                auto_install: true,
                detect_strategy: DetectStrategy::default(),
                node_intercept_globals: false,
                http: HttpOptions::default(),
                pin_latest: None,
                plugins: BTreeMap::default(),
                tools: BTreeMap::default(),
                path: sandbox.path().join(USER_CONFIG_NAME),
            }
        );

        env::remove_var("PROTO_AUTO_CLEAN");
        env::remove_var("PROTO_AUTO_INSTALL");
        env::remove_var("PROTO_NODE_INTERCEPT_GLOBALS");
    }

    #[test]
    fn can_set_plugins() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            "config.toml",
            r#"
[plugins]
foo = "github:moonrepo/foo"
bar = "source:https://moonrepo.dev/path/file.wasm"
"#,
        );

        let config = UserConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(
            config.plugins,
            BTreeMap::from_iter([
                (
                    Id::raw("bar"),
                    PluginLocator::SourceUrl {
                        url: "https://moonrepo.dev/path/file.wasm".into()
                    }
                ),
                (
                    Id::raw("foo"),
                    PluginLocator::GitHub(GitHubLocator {
                        file_prefix: "foo_plugin".into(),
                        repo_slug: "moonrepo/foo".into(),
                        tag: None,
                    })
                ),
            ])
        );
    }

    #[test]
    fn updates_plugin_files_to_absolute() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            "config.toml",
            r#"
[plugins]
foo = "source:../file.wasm"
"#,
        );

        let config = UserConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(
            config.plugins,
            BTreeMap::from_iter([(
                Id::raw("foo"),
                PluginLocator::SourceFile {
                    file: "../file.wasm".into(),
                    path: sandbox.path().join("../file.wasm")
                }
            )])
        );
    }
}
