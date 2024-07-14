mod utils;

use proto_core::{Id, PluginLocator, ProtoConfig};
use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod plugin_remove {
    use super::*;

    #[test]
    fn errors_if_no_local_config() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("plugin").arg("remove").arg("id");
        });

        assert.inner.stderr(predicate::str::contains(
            "No .prototools has been found in current directory.",
        ));
    }

    #[test]
    fn updates_local_file() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path(), |config| {
            config
                .plugins
                .get_or_insert(Default::default())
                .insert(
                    Id::raw("id"),
                    PluginLocator::Url {
                      url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
                    },
                );
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("plugin").arg("remove").arg("id");
            })
            .success();

        let config = load_config(sandbox.path());

        assert!(!config.plugins.contains_key("id"));
    }

    #[test]
    fn updates_global_file() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path().join(".proto"), |config| {
            config
                .plugins
                .get_or_insert(Default::default())
                .insert(
                    Id::raw("id"),
                    PluginLocator::Url {
                      url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
                    },
                );
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("plugin").arg("remove").arg("id").arg("--global");
            })
            .success();

        let config = load_config(sandbox.path().join(".proto"));

        assert!(!config.plugins.contains_key("id"));
    }
}
