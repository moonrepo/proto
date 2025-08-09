use indexmap::IndexMap;
use proto_core::{
    DetectStrategy, EnvVar, PartialEnvVar, PartialProtoSettingsConfig, PinLocation, ProtoConfig,
    ProtoConfigEnvOptions, ProtoFileManager, ToolContext, ToolSpec,
};
use schematic::RegexSetting;
use starbase_sandbox::create_empty_sandbox;
use starbase_utils::json::JsonValue;
use std::collections::BTreeMap;
use std::env;
use version_spec::UnresolvedVersionSpec;
use warpgate::{FileLocator, GitHubLocator, HttpOptions, Id, PluginLocator, UrlLocator};

mod config {
    use super::*;

    #[test]
    #[should_panic(expected = "invalid version value `123`")]
    fn errors_for_non_version_string() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", "node = 123");

        ProtoConfig::load_from(sandbox.path(), false).unwrap();
    }

    #[test]
    #[should_panic(expected = "unknown field `other`")]
    fn errors_for_non_plugins_table() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", "[other]\nkey = 123");

        ProtoConfig::load_from(sandbox.path(), false).unwrap();
    }

    #[test]
    #[should_panic(expected = "proto is a reserved keyword, cannot use as a plugin identifier")]
    fn errors_for_reserved_plugin_words() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
[plugins]
proto = "file://./file.toml"
"#,
        );

        ProtoConfig::load_from(sandbox.path(), false).unwrap();
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
                pin_latest: Some(PinLocation::Global),
                ..Default::default()
            }
        );
    }

    #[test]
    fn can_set_settings_from_env_vars() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", "");

        unsafe {
            env::set_var("PROTO_AUTO_CLEAN", "1");
            env::set_var("PROTO_AUTO_INSTALL", "true");
            env::set_var("PROTO_DETECT_STRATEGY", "prefer-prototools");
            env::set_var("PROTO_PIN_LATEST", "local");
        };

        // Need to use the manager since it runs the finalize process
        let manager = ProtoFileManager::load(sandbox.path(), None, None).unwrap();
        let config = manager.get_merged_config().unwrap();

        assert!(config.settings.auto_clean);
        assert!(config.settings.auto_install);
        assert_eq!(
            config.settings.detect_strategy,
            DetectStrategy::PreferPrototools
        );
        assert_eq!(config.settings.pin_latest, Some(PinLocation::Local));

        unsafe {
            env::remove_var("PROTO_AUTO_CLEAN");
            env::remove_var("PROTO_AUTO_INSTALL");
            env::remove_var("PROTO_DETECT_STRATEGY");
            env::remove_var("PROTO_PIN_LATEST");
        };
    }

    #[test]
    fn can_set_backend_with_id() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", r#""asdf:node" = "20.0.0""#);

        let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();
        let context = ToolContext::parse("asdf:node").unwrap();

        assert_eq!(
            config.versions.unwrap().get(&context).unwrap(),
            &ToolSpec {
                req: UnresolvedVersionSpec::parse("20.0.0").unwrap(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn can_set_env() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
[env]
FOO = true
BAR = false
BAZ_QUX = "abc"
"#,
        );

        let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();

        assert_eq!(config.env.unwrap(), {
            let mut map = IndexMap::new();
            map.insert("FOO".into(), PartialEnvVar::State(true));
            map.insert("BAR".into(), PartialEnvVar::State(false));
            map.insert("BAZ_QUX".into(), PartialEnvVar::Value("abc".into()));
            map
        });
    }

    #[test]
    fn can_set_env_file() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".env", "");
        sandbox.create_file(
            ".prototools",
            r#"
[env]
file = ".env"
KEY = "value"
"#,
        );

        let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();

        assert_eq!(config.env.unwrap(), {
            let mut map = IndexMap::<String, PartialEnvVar>::default();
            map.insert("KEY".into(), PartialEnvVar::Value("value".into()));
            map
        });
        assert_eq!(
            config
                ._env_files
                .unwrap()
                .into_iter()
                .map(|file| file.path)
                .collect::<Vec<_>>(),
            vec![sandbox.path().join(".env")]
        );
    }

    #[test]
    fn can_set_plugins() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
[plugins]
foo = "github://moonrepo/foo"
bar = "https://moonrepo.dev/path/file.wasm"
"#,
        );

        let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();

        assert_eq!(
            config.plugins.unwrap(),
            BTreeMap::from_iter([
                (
                    Id::raw("bar"),
                    PluginLocator::Url(Box::new(UrlLocator {
                        url: "https://moonrepo.dev/path/file.wasm".into()
                    }))
                ),
                (
                    Id::raw("foo"),
                    PluginLocator::GitHub(Box::new(GitHubLocator {
                        repo_slug: "moonrepo/foo".into(),
                        tag: None,
                        project_name: None
                    }))
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
foo = "file://../file.wasm"
"#,
        );

        let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();

        assert_eq!(
            config.plugins.unwrap(),
            BTreeMap::from_iter([(
                Id::raw("foo"),
                PluginLocator::File(Box::new(FileLocator {
                    file: "file://../file.wasm".into(),
                    path: Some(sandbox.path().join("../file.wasm"))
                }))
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
foo = "file://./test.toml"
kebab-case = "file://./camel.toml"
"#,
        );

        let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();

        assert_eq!(
            config.versions.unwrap(),
            BTreeMap::from_iter([
                (
                    ToolContext::parse("node").unwrap(),
                    UnresolvedVersionSpec::parse("12.0.0").unwrap().into()
                ),
                (
                    ToolContext::parse("rust").unwrap(),
                    UnresolvedVersionSpec::Alias("stable".into()).into()
                ),
            ])
        );

        assert_eq!(
            config.plugins.unwrap(),
            BTreeMap::from_iter([
                (
                    Id::raw("foo"),
                    PluginLocator::File(Box::new(FileLocator {
                        file: "file://./test.toml".into(),
                        path: Some(sandbox.path().join("./test.toml"))
                    }))
                ),
                (
                    Id::raw("kebab-case"),
                    PluginLocator::File(Box::new(FileLocator {
                        file: "file://./camel.toml".into(),
                        path: Some(sandbox.path().join("./camel.toml"))
                    }))
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
            ToolContext::parse("node").unwrap(),
            UnresolvedVersionSpec::parse("12.0.0").unwrap().into(),
        );
        versions.insert(
            ToolContext::parse("rust").unwrap(),
            UnresolvedVersionSpec::Alias("stable".into()).into(),
        );

        let plugins = config.plugins.get_or_insert(Default::default());

        plugins.insert(
            Id::raw("foo"),
            PluginLocator::File(Box::new(FileLocator {
                file: "./test.toml".into(),
                path: Some(sandbox.path().join("./test.toml")),
            })),
        );

        let path = ProtoConfig::save_partial_to(sandbox.path(), config).unwrap();

        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            r#"node = "12.0.0"
rust = "stable"

[plugins]
foo = "file://./test.toml"
"#,
        );
    }

    mod envs {
        use super::*;

        fn new_env_options() -> ProtoConfigEnvOptions {
            ProtoConfigEnvOptions {
                check_process: true,
                include_shared: true,
                tool_id: None,
            }
        }

        #[test]
        #[should_panic(expected = "MissingEnvFile")]
        fn errors_if_file_missing() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
[env]
file = ".env"
"#,
            );

            ProtoFileManager::load(sandbox.path(), None, None)
                .unwrap()
                .get_merged_config()
                .unwrap();
        }

        #[test]
        #[should_panic(expected = "FailedParseEnvFile")]
        fn errors_if_parse_fails() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".env",
                r#"
.KEY={invalid}
"#,
            );
            sandbox.create_file(
                ".prototools",
                r#"
[env]
file = ".env"
"#,
            );

            ProtoFileManager::load(sandbox.path(), None, None)
                .unwrap()
                .get_merged_config()
                .unwrap()
                .get_env_vars(new_env_options())
                .unwrap();
        }

        #[test]
        fn merges_vars_and_files() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".env",
                r#"
KEY1 = "file1"
KEY3 = "file3"
"#,
            );
            sandbox.create_file(
                ".prototools",
                r#"
[env]
file = ".env"
KEY1 = "value1"
KEY2 = "value2"
"#,
            );

            let config = ProtoFileManager::load(sandbox.path(), None, None)
                .unwrap()
                .get_merged_config()
                .unwrap()
                .to_owned();

            assert_eq!(config.get_env_vars(new_env_options()).unwrap(), {
                let mut map = IndexMap::<String, Option<String>>::default();
                map.insert("KEY1".into(), Some("value1".into()));
                map.insert("KEY2".into(), Some("value2".into()));
                map.insert("KEY3".into(), Some("file3".into()));
                map
            });
        }

        #[test]
        fn child_file_overwrites_parent() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".env",
                r#"
KEY = "parent"
"#,
            );
            sandbox.create_file(
                ".prototools",
                r#"
[env]
file = ".env"
"#,
            );
            sandbox.create_file(
                "child/.env",
                r#"
KEY = "child"
"#,
            );
            sandbox.create_file(
                "child/.prototools",
                r#"
[env]
file = ".env"
"#,
            );

            let config = ProtoFileManager::load(sandbox.path().join("child"), None, None)
                .unwrap()
                .get_merged_config()
                .unwrap()
                .to_owned();

            assert_eq!(config.get_env_vars(new_env_options()).unwrap(), {
                let mut map = IndexMap::<String, Option<String>>::default();
                map.insert("KEY".into(), Some("child".into()));
                map
            });
        }

        #[test]
        fn files_can_substitute_from_self() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".env",
                r#"
OTHER = "abc"
KEY = "other=${OTHER}"
"#,
            );
            sandbox.create_file(
                ".prototools",
                r#"
[env]
file = ".env"
"#,
            );

            let config = ProtoFileManager::load(sandbox.path(), None, None)
                .unwrap()
                .get_merged_config()
                .unwrap()
                .to_owned();

            assert_eq!(config.get_env_vars(new_env_options()).unwrap(), {
                let mut map = IndexMap::<String, Option<String>>::default();
                map.insert("OTHER".into(), Some("abc".into()));
                map.insert("KEY".into(), Some("other=abc".into()));
                map
            });
        }

        //         #[test]
        //         fn files_can_substitute_from_process() {
        //             let sandbox = create_empty_sandbox();
        //             sandbox.create_file(
        //                 ".env",
        //                 r#"
        // KEY = "process=${PROCESS_KEY}"
        // "#,
        //             );
        //             sandbox.create_file(
        //                 ".prototools",
        //                 r#"
        // [env]
        // file = ".env"
        // "#,
        //             );

        //             env::set_var("PROCESS_KEY", "abc");

        //             let config = ProtoConfigManager::load(sandbox.path(), None, None)
        //                 .unwrap()
        //                 .get_merged_config()
        //                 .unwrap()
        //                 .to_owned();

        //             env::remove_var("PROCESS_KEY");

        //             assert_eq!(config.get_env_vars(None).unwrap(), {
        //                 let mut map = IndexMap::<String, Option<String>>::default();
        //                 map.insert("KEY".into(), Some("process=abc".into()));
        //                 map
        //             });
        //         }

        #[test]
        fn vars_can_substitute_from_files() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".env",
                r#"
FILE=file
"#,
            );
            sandbox.create_file(
                ".prototools",
                r#"
[env]
file = ".env"
KEY = "from=${FILE}"
"#,
            );

            let config = ProtoFileManager::load(sandbox.path(), None, None)
                .unwrap()
                .get_merged_config()
                .unwrap()
                .to_owned();

            assert_eq!(config.get_env_vars(new_env_options()).unwrap(), {
                let mut map = IndexMap::<String, Option<String>>::default();
                map.insert("FILE".into(), Some("file".into()));
                map.insert("KEY".into(), Some("from=file".into()));
                map
            });
        }
    }

    mod builtins {
        use super::*;
        use proto_core::BuiltinPlugins;
        use schematic::Config;

        #[test]
        fn can_enable() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
[settings]
builtin-plugins = true
"#,
            );

            let config =
                ProtoConfig::from_partial(ProtoConfig::load_from(sandbox.path(), false).unwrap());

            assert_eq!(
                config.settings.builtin_plugins,
                BuiltinPlugins::Enabled(true)
            );

            assert_eq!(config.builtin_plugins().len(), 20);
        }

        #[test]
        fn can_enable_with_list() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
[settings]
builtin-plugins = ["node", "go"]
"#,
            );

            let config =
                ProtoConfig::from_partial(ProtoConfig::load_from(sandbox.path(), false).unwrap());

            assert_eq!(
                config.settings.builtin_plugins,
                BuiltinPlugins::Allowed(vec!["node".into(), "go".into()])
            );

            assert_eq!(config.builtin_plugins().len(), 8);
            assert_eq!(
                config.builtin_plugins().keys().collect::<Vec<_>>(),
                [
                    "go",
                    "internal-schema",
                    "moonbase",
                    "moonstone",
                    "node",
                    "proto",
                    "protoform",
                    "protostar",
                ]
            );
        }

        #[test]
        fn can_disable() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
[settings]
builtin-plugins = false
"#,
            );

            let config =
                ProtoConfig::from_partial(ProtoConfig::load_from(sandbox.path(), false).unwrap());

            assert_eq!(
                config.settings.builtin_plugins,
                BuiltinPlugins::Enabled(false)
            );

            assert_eq!(config.builtin_plugins().len(), 6);
        }

        #[test]
        fn can_disable_with_list() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
[settings]
builtin-plugins = []
"#,
            );

            let config =
                ProtoConfig::from_partial(ProtoConfig::load_from(sandbox.path(), false).unwrap());

            assert_eq!(
                config.settings.builtin_plugins,
                BuiltinPlugins::Allowed(vec![])
            );

            assert_eq!(config.builtin_plugins().len(), 6);
        }
    }

    mod tool_config {
        use super::*;
        use rustc_hash::FxHashMap;

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
                &FxHashMap::from_iter([
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

            let config = ProtoFileManager::load(sandbox.path().join("a/b"), None, None)
                .unwrap()
                .get_merged_config()
                .unwrap()
                .to_owned();

            assert_eq!(
                config.tools.get("node").unwrap().config,
                FxHashMap::from_iter([
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

            let config = ProtoFileManager::load(sandbox.path().join("a/b"), None, None)
                .unwrap()
                .get_merged_config()
                .unwrap()
                .to_owned();

            assert_eq!(
                config.tools.get("node").unwrap().aliases,
                BTreeMap::from_iter([
                    (
                        "stable".to_owned(),
                        UnresolvedVersionSpec::parse("1.0.0").unwrap().into()
                    ),
                    (
                        "value".to_owned(),
                        UnresolvedVersionSpec::parse("1.2.3").unwrap().into()
                    ),
                ])
            );
        }

        #[test]
        fn merges_env_vars() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                "a/b/.prototools",
                r#"
[tools.node.env]
NODE_ENV = "production"
"#,
            );
            sandbox.create_file(
                "a/.prototools",
                r#"
[env]
APP_NAME = "middle"

[tools.node.env]
NODE_ENV = "development"
"#,
            );
            sandbox.create_file(
                ".prototools",
                r#"
[env]
APP_TYPE = "ssg"

[tools.node.env]
NODE_PATH = false
"#,
            );

            let config = ProtoFileManager::load(sandbox.path().join("a/b"), None, None)
                .unwrap()
                .get_merged_config()
                .unwrap()
                .to_owned();

            assert_eq!(config.env, {
                let mut map = IndexMap::new();
                map.insert("APP_NAME".into(), EnvVar::Value("middle".into()));
                map.insert("APP_TYPE".into(), EnvVar::Value("ssg".into()));
                map
            });

            assert_eq!(config.tools.get("node").unwrap().env, {
                let mut map = IndexMap::new();
                map.insert("NODE_ENV".into(), EnvVar::Value("production".into()));
                map.insert("NODE_PATH".into(), EnvVar::State(false));
                map
            });
        }

        #[test]
        fn gathers_env_files() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a/b/.env.b", "");
            sandbox.create_file("a/b/.env.tool-b", "");
            sandbox.create_file(
                "a/b/.prototools",
                r#"
[env]
file = ".env.b"

[tools.node.env]
file = ".env.tool-b"
"#,
            );
            sandbox.create_file("a/.env.a", "");
            sandbox.create_file("a/.env.tool-a", "");
            sandbox.create_file(
                "a/.prototools",
                r#"
[env]
file = ".env.a"

[tools.node.env]
file = ".env.tool-a"
"#,
            );
            sandbox.create_file(".env", "");
            sandbox.create_file(".env.tool", "");
            sandbox.create_file(
                ".prototools",
                r#"
[env]
file = ".env"

[tools.node.env]
file = ".env.tool"
"#,
            );

            let config = ProtoFileManager::load(sandbox.path().join("a/b"), None, None)
                .unwrap()
                .get_merged_config()
                .unwrap()
                .to_owned();

            assert_eq!(config.env, IndexMap::<String, EnvVar>::default());
            assert_eq!(
                config
                    .get_env_files(ProtoConfigEnvOptions {
                        check_process: true,
                        include_shared: true,
                        tool_id: Some(Id::raw("node")),
                    })
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>(),
                vec![
                    sandbox.path().join(".env"),
                    sandbox.path().join(".env.tool"),
                    sandbox.path().join("a/.env.a"),
                    sandbox.path().join("a/.env.tool-a"),
                    sandbox.path().join("a/b/.env.b"),
                    sandbox.path().join("a/b/.env.tool-b"),
                ]
            );
        }
    }

    mod url_rewrites {
        use schematic::Config;

        use super::*;

        #[test]
        fn can_set_regex_setting() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
[settings.url-rewrites]
"github.com/(\\w+)/(\\w+)" = "replaced.com/$1/$2"
"#,
            );

            let config = ProtoConfig::load_from(sandbox.path(), false).unwrap();

            assert_eq!(
                config.settings.unwrap(),
                PartialProtoSettingsConfig {
                    url_rewrites: Some(IndexMap::from_iter([(
                        RegexSetting::try_from("github.com/(\\w+)/(\\w+)".to_string()).unwrap(),
                        "replaced.com/$1/$2".into()
                    )])),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn rewrites_urls() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
[settings.url-rewrites]
"github.com/(\\w+)/(\\w+)" = "replaced.com/$1/$2"
"mo+n" = "lunar"
"#,
            );

            let config =
                ProtoConfig::from_partial(ProtoConfig::load_from(sandbox.path(), false).unwrap());

            assert_eq!(
                config.rewrite_url("https://github.com/moonrepo/moon/releases/download/v1.37.1/moon-x86_64-apple-darwin"),
                "https://replaced.com/lunarrepo/lunar/releases/download/v1.37.1/lunar-x86_64-apple-darwin",
            );
        }
    }
}
