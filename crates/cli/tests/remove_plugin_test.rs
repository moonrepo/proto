mod utils;

use proto_core::{Id, PluginLocator, ToolsConfig, UserConfig};
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeMap;
use utils::*;

mod remove_plugin {
    use super::*;

    #[test]
    fn errors_if_no_local_config() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("remove-plugin").arg("id").assert();

        assert.stderr(predicate::str::contains(
            "No .prototools has been found in current directory.",
        ));
    }

    #[test]
    fn updates_local_file() {
        let sandbox = create_empty_sandbox();

        let mut config = ToolsConfig::load_from(sandbox.path()).unwrap();
        config.plugins.insert(Id::raw("id"), PluginLocator::SourceUrl {
            url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
        });
        config.save().unwrap();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("remove-plugin").arg("id").assert().success();

        let config = ToolsConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(config.plugins, BTreeMap::new());
    }

    #[test]
    fn updates_global_file() {
        let sandbox = create_empty_sandbox();

        let mut config = UserConfig::load_from(sandbox.path()).unwrap();
        config.plugins.insert(Id::raw("id"), PluginLocator::SourceUrl {
            url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
        });
        config.save().unwrap();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("remove-plugin")
            .arg("id")
            .arg("--global")
            .assert()
            .success();

        let config = UserConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(config.plugins, BTreeMap::new());
    }
}
