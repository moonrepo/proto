mod utils;

use proto_core::PluginLocator;
use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod plugin_add {
    use super::*;

    #[test]
    fn errors_invalid_locator() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("plugin")
                .arg("add")
                .arg("id")
                .arg("some-fake-value");
        });

        assert
            .inner
            .stderr(predicate::str::contains("Missing plugin protocol"));
    }

    #[test]
    fn updates_local_file() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join(".prototools");

        assert!(!config_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("plugin")
                    .arg("add")
                    .arg("id")
                    .arg("https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path());

        assert_eq!(
            config.plugins.get("id").unwrap(),
            &PluginLocator::Url {
                url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
            }
        );
    }

    #[test]
    fn updates_global_file() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join(".proto/.prototools");

        assert!(!config_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("plugin")
                    .arg("add")
                    .arg("id")
                    .arg("https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm")
                    .arg("--global");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config.plugins.get("id").unwrap(),
            &PluginLocator::Url {
                url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
            }
        );
    }
}
