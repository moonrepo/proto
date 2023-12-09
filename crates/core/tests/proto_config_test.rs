use proto_core::{
    DetectStrategy, PartialProtoSettingsConfig, PinType, ProtoConfig, ProtoConfigManager,
};
use schematic::ConfigError;
use starbase_sandbox::create_empty_sandbox;
use starbase_utils::json::JsonValue;
use std::collections::BTreeMap;
use std::env;
use version_spec::UnresolvedVersionSpec;
use warpgate::{GitHubLocator, HttpOptions, Id, PluginLocator};

fn handle_error(report: miette::Report) {
    panic!(
        "{}",
        report
            .downcast_ref::<ConfigError>()
            .unwrap()
            .to_full_string()
    );
}

mod proto_config {
    use super::*;

    #[test]
    #[should_panic(expected = "invalid version value `123`")]
    fn errors_for_non_version_string() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", "node = 123");

        handle_error(ProtoConfig::load_from(sandbox.path(), false).unwrap_err());
    }

    #[test]
    #[should_panic(expected = "unknown field `other`")]
    fn errors_for_non_plugins_table() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", "[other]\nkey = 123");

        handle_error(ProtoConfig::load_from(sandbox.path(), false).unwrap_err());
    }

    #[test]
    #[should_panic(expected = "must be a valid kebab-case string.")]
    fn errors_for_non_kebab_id() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", "fooBar = \"1.2.3\"");

        handle_error(ProtoConfig::load_from(sandbox.path(), false).unwrap_err());
    }

    #[test]
    fn can_set_settings() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
[settings]
auto-clean = true
auto-install = true
pin-latest = "global"
"#,
        );

        let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();

        assert_eq!(
            config.settings.unwrap(),
            PartialProtoSettingsConfig {
                auto_clean: Some(true),
                auto_install: Some(true),
                pin_latest: Some(PinType::Global),
                ..Default::default()
            }
        );
    }

    #[test]
    fn can_set_settings_from_env_vars() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", "");

        env::set_var("PROTO_AUTO_CLEAN", "1");
        env::set_var("PROTO_AUTO_INSTALL", "true");
        env::set_var("PROTO_DETECT_STRATEGY", "prefer-prototools");
        env::set_var("PROTO_PIN_LATEST", "local");

        // Need to use the manager since it runs the finalize process
        let manager = ProtoConfigManager::load(sandbox.path(), None).unwrap();
        let config = manager.get_merged_config().unwrap();

        assert!(config.settings.auto_clean);
        assert!(config.settings.auto_install);
        assert_eq!(
            config.settings.detect_strategy,
            DetectStrategy::PreferPrototools
        );
        assert_eq!(config.settings.pin_latest, Some(PinType::Local));

        env::remove_var("PROTO_AUTO_CLEAN");
        env::remove_var("PROTO_AUTO_INSTALL");
        env::remove_var("PROTO_DETECT_STRATEGY");
        env::remove_var("PROTO_PIN_LATEST");
    }

    #[test]
    fn can_set_plugins() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
[plugins]
foo = "github:moonrepo/foo"
bar = "source:https://moonrepo.dev/path/file.wasm"
"#,
        );

        let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();

        assert_eq!(
            config.plugins.unwrap(),
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
            ".prototools",
            r#"
[plugins]
foo = "source:../file.wasm"
"#,
        );

        let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();

        assert_eq!(
            config.plugins.unwrap(),
            BTreeMap::from_iter([(
                Id::raw("foo"),
                PluginLocator::SourceFile {
                    file: "../file.wasm".into(),
                    path: sandbox.path().join("../file.wasm")
                }
            )])
        );
    }

    #[test]
    fn updates_root_cert_to_absolute() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
[settings.http]
root-cert = "../cert.pem"
"#,
        );

        let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();

        assert_eq!(
            config.settings.unwrap().http.unwrap(),
            HttpOptions {
                root_cert: Some(sandbox.path().join("../cert.pem")),
                ..Default::default()
            }
        );
    }

    #[test]
    fn parses_plugins_table() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
    node = "12.0.0"
    rust = "stable"

    [plugins]
    foo = "source:./test.toml"
    kebab-case = "source:./camel.toml"
    "#,
        );

        let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();

        assert_eq!(
            config.versions.unwrap(),
            BTreeMap::from_iter([
                (
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("12.0.0").unwrap()
                ),
                (
                    Id::raw("rust"),
                    UnresolvedVersionSpec::Alias("stable".into())
                ),
            ])
        );

        assert_eq!(
            config.plugins.unwrap(),
            BTreeMap::from_iter([
                (
                    Id::raw("foo"),
                    PluginLocator::SourceFile {
                        file: "./test.toml".into(),
                        path: sandbox.path().join("./test.toml")
                    }
                ),
                (
                    Id::raw("kebab-case"),
                    PluginLocator::SourceFile {
                        file: "./camel.toml".into(),
                        path: sandbox.path().join("./camel.toml")
                    }
                )
            ])
        );
    }

    #[test]
    fn formats_plugins_table() {
        let sandbox = create_empty_sandbox();
        let mut config = ProtoConfig::load_from(sandbox.path(), false).unwrap();
        let versions = config.versions.get_or_insert(Default::default());

        versions.insert(
            Id::raw("node"),
            UnresolvedVersionSpec::parse("12.0.0").unwrap(),
        );
        versions.insert(
            Id::raw("rust"),
            UnresolvedVersionSpec::Alias("stable".into()),
        );

        let plugins = config.plugins.get_or_insert(Default::default());

        plugins.insert(
            Id::raw("foo"),
            PluginLocator::SourceFile {
                file: "./test.toml".into(),
                path: sandbox.path().join("./test.toml"),
            },
        );

        let path = ProtoConfig::save_to(sandbox.path(), config).unwrap();

        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            r#"node = "12.0.0"
rust = "stable"

[plugins]
foo = "source:./test.toml"
"#,
        );
    }

    mod tool_config {
        use super::*;

        #[test]
        fn can_set_extra_settings() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
[tools.node]
bundled-npm = "bundled"
intercept-globals = false
"#,
            );

            let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();

            assert_eq!(
                config
                    .tools
                    .unwrap()
                    .get("node")
                    .unwrap()
                    .config
                    .as_ref()
                    .unwrap(),
                &BTreeMap::from_iter([
                    (
                        "bundled-npm".to_owned(),
                        JsonValue::String("bundled".into())
                    ),
                    ("intercept-globals".to_owned(), JsonValue::Bool(false)),
                ])
            );
        }

        #[test]
        fn merges_plugin_settings() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                "a/b/.prototools",
                r#"
[tools.node]
value = "b"
"#,
            );
            sandbox.create_file(
                "a/.prototools",
                r#"
[tools.node]
depth = 1
"#,
            );
            sandbox.create_file(
                ".prototools",
                r#"
[tools.node]
value = "root"
"#,
            );

            let config = ProtoConfigManager::load(sandbox.path().join("a/b"), None)
                .unwrap()
                .get_merged_config()
                .unwrap()
                .to_owned();

            assert_eq!(
                config.tools.get("node").unwrap().config,
                BTreeMap::from_iter([
                    ("value".to_owned(), JsonValue::String("b".into())),
                    ("depth".to_owned(), JsonValue::from(1)),
                ])
            );
        }

        #[test]
        fn merges_aliases() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                "a/b/.prototools",
                r#"
[tools.node.aliases]
value = "1.2.3"
"#,
            );
            sandbox.create_file(
                "a/.prototools",
                r#"
[tools.node.aliases]
stable = "1.0.0"
"#,
            );
            sandbox.create_file(
                ".prototools",
                r#"
[tools.node.aliases]
value = "4.5.6"
"#,
            );

            let config = ProtoConfigManager::load(sandbox.path().join("a/b"), None)
                .unwrap()
                .get_merged_config()
                .unwrap()
                .to_owned();

            assert_eq!(
                config.tools.get("node").unwrap().aliases,
                BTreeMap::from_iter([
                    (
                        "stable".to_owned(),
                        UnresolvedVersionSpec::parse("1.0.0").unwrap()
                    ),
                    (
                        "value".to_owned(),
                        UnresolvedVersionSpec::parse("1.2.3").unwrap()
                    ),
                ])
            );
        }
    }
}

mod proto_config_manager {
    use super::*;

    #[test]
    fn merges_traversing_upwards() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(
            "one/two/three/.prototools",
            r#"
node = "1.2.3"

[plugins]
node = "source:./node.toml"
"#,
        );

        sandbox.create_file(
            "one/two/.prototools",
            r#"
[plugins]
bun = "source:../bun.wasm"
"#,
        );

        sandbox.create_file(
            "one/.prototools",
            r#"
bun = "4.5.6"

[plugins]
node = "source:../node.toml"
"#,
        );

        sandbox.create_file(
            ".prototools",
            r#"
node = "7.8.9"
deno = "7.8.9"
"#,
        );

        let manager = ProtoConfigManager::load(sandbox.path().join("one/two/three"), None).unwrap();
        let config = manager.get_merged_config().unwrap();

        assert_eq!(
            config.versions,
            BTreeMap::from_iter([
                (
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("1.2.3").unwrap()
                ),
                (
                    Id::raw("bun"),
                    UnresolvedVersionSpec::parse("4.5.6").unwrap()
                ),
                (
                    Id::raw("deno"),
                    UnresolvedVersionSpec::parse("7.8.9").unwrap()
                ),
            ])
        );

        assert_eq!(
            config.plugins.get("node").unwrap(),
            &PluginLocator::SourceFile {
                file: "./node.toml".into(),
                path: sandbox.path().join("one/two/three/./node.toml")
            }
        );

        assert_eq!(
            config.plugins.get("bun").unwrap(),
            &PluginLocator::SourceFile {
                file: "../bun.wasm".into(),
                path: sandbox.path().join("one/two/../bun.wasm")
            }
        );
    }

    #[test]
    fn merges_traversing_upwards_without_global() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(
            "one/two/three/.prototools",
            r#"
node = "1.2.3"
"#,
        );

        sandbox.create_file(
            ".prototools",
            r#"
node = "7.8.9"
deno = "7.8.9"
"#,
        );

        sandbox.create_file(
            ".proto/.prototools",
            r#"
bun = "1.2.3"
"#,
        );

        let manager = ProtoConfigManager::load(sandbox.path().join("one/two/three"), None).unwrap();
        let config = manager.get_merged_config_without_global().unwrap();

        assert_eq!(
            config.versions,
            BTreeMap::from_iter([
                (
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("1.2.3").unwrap()
                ),
                (
                    Id::raw("deno"),
                    UnresolvedVersionSpec::parse("7.8.9").unwrap()
                ),
            ])
        );
    }

    #[test]
    fn merges_local_only() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(
            "one/two/three/.prototools",
            r#"
node = "1.2.3"
"#,
        );

        sandbox.create_file(
            ".prototools",
            r#"
node = "7.8.9"
deno = "7.8.9"
"#,
        );

        sandbox.create_file(
            ".proto/.prototools",
            r#"
bun = "1.2.3"
"#,
        );

        let manager = ProtoConfigManager::load(sandbox.path().join("one/two/three"), None).unwrap();
        let config = manager.get_local_config().unwrap();

        assert_eq!(
            config.versions,
            BTreeMap::from_iter([(
                Id::raw("node"),
                UnresolvedVersionSpec::parse("1.2.3").unwrap()
            ),])
        );
    }
}
