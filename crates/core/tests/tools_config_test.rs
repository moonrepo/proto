use assert_fs::prelude::{FileWriteStr, PathChild};
use proto_core::{PluginLocator, ToolsConfig};
use rustc_hash::FxHashMap;

#[test]
#[should_panic(expected = "InvalidConfig")]
fn errors_for_non_version_string() {
    let fixture = assert_fs::TempDir::new().unwrap();
    fixture
        .child(".prototools")
        .write_str("node = 123")
        .unwrap();

    ToolsConfig::load_from(fixture.path()).unwrap();
}

#[test]
fn supports_plugins_table() {
    let fixture = assert_fs::TempDir::new().unwrap();
    fixture
        .child(".prototools")
        .write_str(
            r#"
node = "12.0.0"

[plugins]
foo = "schema:./test.toml"
"#,
        )
        .unwrap();

    let config = ToolsConfig::load_from(fixture.path()).unwrap();

    assert_eq!(
        config.tools,
        FxHashMap::from_iter([("node".into(), "12.0.0".into())])
    );

    assert_eq!(
        config.plugins,
        FxHashMap::from_iter([("foo".into(), PluginLocator::Schema("./test.toml".into()))])
    );
}
