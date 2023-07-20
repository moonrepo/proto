use proto_core::{PluginLocator, ToolsConfig};
use rustc_hash::FxHashMap;
use starbase_sandbox::create_empty_sandbox;

#[test]
#[should_panic(expected = "InvalidConfig")]
fn errors_for_non_version_string() {
    let fixture = create_empty_sandbox();
    fixture.create_file(".prototools", "node = 123");

    ToolsConfig::load_from(fixture.path()).unwrap();
}

#[test]
#[should_panic(expected = "InvalidConfig")]
fn errors_for_non_plugins_table() {
    let fixture = create_empty_sandbox();
    fixture.create_file(".prototools", "[other]\nkey = 123");

    ToolsConfig::load_from(fixture.path()).unwrap();
}

#[test]
fn parses_plugins_table() {
    let fixture = create_empty_sandbox();
    fixture.create_file(
        ".prototools",
        r#"
node = "12.0.0"

[plugins]
foo = "source:./test.toml"
camelCase = "source:./camel.toml"
"#,
    );

    let config = ToolsConfig::load_from(fixture.path()).unwrap();

    assert_eq!(
        config.tools,
        FxHashMap::from_iter([("node".into(), "12.0.0".into())])
    );

    assert_eq!(
        config.plugins,
        FxHashMap::from_iter([
            (
                "foo".into(),
                PluginLocator::SourceFile {
                    file: "./test.toml".into(),
                    path: fixture.path().join("./test.toml")
                }
            ),
            (
                "camel-case".into(),
                PluginLocator::SourceFile {
                    file: "./camel.toml".into(),
                    path: fixture.path().join("./camel.toml")
                }
            )
        ])
    );
}

#[test]
fn formats_plugins_table() {
    let fixture = create_empty_sandbox();

    let mut config = ToolsConfig::load_from(fixture.path()).unwrap();
    config.tools.insert("node".into(), "12.0.0".into());
    config.plugins.insert(
        "foo".into(),
        PluginLocator::SourceFile {
            file: "./test.toml".into(),
            path: fixture.path().join("./test.toml"),
        },
    );
    config.save().unwrap();

    assert_eq!(
        std::fs::read_to_string(config.path).unwrap(),
        r#"node = "12.0.0"

[plugins]
foo = "source:./test.toml"
"#,
    );
}

#[test]
fn merges_traversing_upwards() {
    let fixture = create_empty_sandbox();

    fixture.create_file(
        "one/two/three/.prototools",
        r#"
node = "1.2.3"

[plugins]
node = "source:./node.toml"
"#,
    );

    fixture.create_file(
        "one/two/.prototools",
        r#"
[plugins]
bun = "source:../bun.wasm"
"#,
    );

    fixture.create_file(
        "one/.prototools",
        r#"
bun = "4.5.6"

[plugins]
node = "source:../node.toml"
"#,
    );

    fixture.create_file(
        ".prototools",
        r#"
node = "7.8.9"
deno = "7.8.9"
"#,
    );

    let config = ToolsConfig::load_upwards_from(fixture.path().join("one/two/three")).unwrap();

    assert_eq!(
        config.tools,
        FxHashMap::from_iter([
            ("node".into(), "1.2.3".into()),
            ("bun".into(), "4.5.6".into()),
            ("deno".into(), "7.8.9".into()),
        ])
    );

    assert_eq!(
        config.plugins,
        FxHashMap::from_iter([
            (
                "node".into(),
                PluginLocator::SourceFile {
                    file: "./node.toml".into(),
                    path: fixture.path().join("one/two/three/./node.toml")
                }
            ),
            (
                "bun".into(),
                PluginLocator::SourceFile {
                    file: "../bun.wasm".into(),
                    path: fixture.path().join("one/two/../bun.wasm")
                }
            )
        ])
    );
}
