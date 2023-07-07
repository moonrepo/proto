use proto_core::{PluginLocation, PluginLocator, ToolsConfig};
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
                PluginLocator::Source(PluginLocation::File("./test.toml".into()))
            ),
            (
                "camel-case".into(),
                PluginLocator::Source(PluginLocation::File("./camel.toml".into()))
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
        PluginLocator::Source(PluginLocation::File("./test.toml".into())),
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
