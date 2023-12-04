mod utils;

use proto_core::{Id, PluginLocator, ProtoConfig};
use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod tool_remove {
    use super::*;

    #[test]
    fn errors_if_no_local_config() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("tool").arg("remove").arg("id").assert();

        assert.stderr(predicate::str::contains(
            "No .prototools has been found in current directory.",
        ));
    }

    #[test]
    fn updates_local_file() {
        let sandbox = create_empty_sandbox();

        ProtoConfig::update(sandbox.path(), |config| {
            config
                .plugins
                .get_or_insert(Default::default())
                .insert(
                    Id::raw("id"),
                    PluginLocator::SourceUrl {
                      url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
                    },
                );
        })
        .unwrap();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("tool").arg("remove").arg("id").assert().success();

        let config = load_config(sandbox.path());

        assert!(!config.plugins.contains_key("id"));
    }

    #[test]
    fn updates_global_file() {
        let sandbox = create_empty_sandbox();

        ProtoConfig::update(sandbox.path().join(".proto"), |config| {
            config
                .plugins
                .get_or_insert(Default::default())
                .insert(
                    Id::raw("id"),
                    PluginLocator::SourceUrl {
                      url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
                    },
                );
        })
        .unwrap();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("tool")
            .arg("remove")
            .arg("id")
            .arg("--global")
            .assert()
            .success();

        let config = load_config(sandbox.path().join(".proto"));

        assert!(!config.plugins.contains_key("id"));
    }
}
