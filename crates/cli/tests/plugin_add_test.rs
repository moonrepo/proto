mod utils;

use proto_core::{Id, PluginLocator, ToolsConfig, UserConfig, TOOLS_CONFIG_NAME, USER_CONFIG_NAME};
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeMap;
use utils::*;

mod plugin_add {
    use super::*;

    #[test]
    fn errors_invalid_locator() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("add-plugin")
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
        let config_file = sandbox.path().join(TOOLS_CONFIG_NAME);

        assert!(!config_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("add-plugin")
            .arg("id")
            .arg("source:https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm")
            .assert()
            .success();

        assert!(config_file.exists());

        let manifest = ToolsConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(
            manifest.plugins,
            BTreeMap::from_iter([(
                Id::raw("id"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
                }
            )])
        );
    }

    #[test]
    fn updates_global_file() {
        let sandbox = create_empty_sandbox();
        let config_file = sandbox.path().join(USER_CONFIG_NAME);

        assert!(!config_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("add-plugin")
            .arg("id")
            .arg("source:https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm")
            .arg("--global")
            .assert()
            .success();

        assert!(config_file.exists());

        let manifest = UserConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(
            manifest.plugins,
            BTreeMap::from_iter([(
                Id::raw("id"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
                }
            )])
        );
    }
}
