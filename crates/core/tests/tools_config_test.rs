use proto_core::{AliasOrVersion, ToolsConfig};
use starbase_sandbox::create_empty_sandbox;
use std::collections::BTreeMap;
use std::str::FromStr;
use warpgate::PluginLocator;

mod tools_config {
    use super::*;

    #[test]
    #[should_panic(expected = "invalid type: integer `123`, expected a string")]
    fn errors_for_non_version_string() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", "node = 123");

        ToolsConfig::load_from(sandbox.path()).unwrap();
    }

    #[test]
    #[should_panic(expected = "invalid type: map, expected a string")]
    fn errors_for_non_plugins_table() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", "[other]\nkey = 123");

        ToolsConfig::load_from(sandbox.path()).unwrap();
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
camelCase = "source:./camel.toml"
"#,
        );

        let config = ToolsConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(
            config.tools,
            BTreeMap::from_iter([
                ("node".into(), AliasOrVersion::from_str("12.0.0").unwrap()),
                ("rust".into(), AliasOrVersion::Alias("stable".into())),
            ])
        );

        assert_eq!(
            config.plugins,
            BTreeMap::from_iter([
                (
                    "foo".into(),
                    PluginLocator::SourceFile {
                        file: "./test.toml".into(),
                        path: sandbox.path().join("./test.toml")
                    }
                ),
                (
                    "camelCase".into(),
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
        let mut config = ToolsConfig::load_from(sandbox.path()).unwrap();

        config
            .tools
            .insert("node".into(), AliasOrVersion::from_str("12.0.0").unwrap());
        config
            .tools
            .insert("rust".into(), AliasOrVersion::Alias("stable".into()));

        config.plugins.insert(
            "foo".into(),
            PluginLocator::SourceFile {
                file: "./test.toml".into(),
                path: sandbox.path().join("./test.toml"),
            },
        );
        config.save().unwrap();

        assert_eq!(
            std::fs::read_to_string(config.path).unwrap(),
            r#"node = "12.0.0"
rust = "stable"

[plugins]
foo = "source:./test.toml"
"#,
        );
    }

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

        let config = ToolsConfig::load_upwards_from(sandbox.path().join("one/two/three")).unwrap();

        assert_eq!(
            config.tools,
            BTreeMap::from_iter([
                ("node".into(), AliasOrVersion::parse("1.2.3").unwrap()),
                ("bun".into(), AliasOrVersion::parse("4.5.6").unwrap()),
                ("deno".into(), AliasOrVersion::parse("7.8.9").unwrap()),
            ])
        );

        assert_eq!(
            config.plugins,
            BTreeMap::from_iter([
                (
                    "node".into(),
                    PluginLocator::SourceFile {
                        file: "./node.toml".into(),
                        path: sandbox.path().join("one/two/three/./node.toml")
                    }
                ),
                (
                    "bun".into(),
                    PluginLocator::SourceFile {
                        file: "../bun.wasm".into(),
                        path: sandbox.path().join("one/two/../bun.wasm")
                    }
                )
            ])
        );
    }
}
