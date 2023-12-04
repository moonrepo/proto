mod utils;

use proto_core::PluginLocator;
use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod tool_add {
    use super::*;

    #[test]
    fn errors_invalid_locator() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("tool")
            .arg("add")
            .arg("id")
            .arg("some-fake-value")
            .assert();

        assert.stderr(predicate::str::contains(
            "Missing plugin scope or location.",
        ));
    }

    #[test]
    fn updates_local_file() {
        let sandbox = create_empty_sandbox();
        let config_file = sandbox.path().join(".prototools");

        assert!(!config_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("tool")
            .arg("add")
            .arg("id")
            .arg("source:https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm")
            .assert()
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path());

        assert_eq!(
            config.plugins.get("id").unwrap(),
            &PluginLocator::SourceUrl {
                url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
            }
        );
    }

    #[test]
    fn updates_global_file() {
        let sandbox = create_empty_sandbox();
        let config_file = sandbox.path().join(".proto/.prototools");

        assert!(!config_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("tool")
            .arg("add")
            .arg("id")
            .arg("source:https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm")
            .arg("--global")
            .assert()
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config.plugins.get("id").unwrap(),
            &PluginLocator::SourceUrl {
                url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
            }
        );
    }
}
