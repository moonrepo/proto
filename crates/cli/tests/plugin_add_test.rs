mod utils;

use proto_core::{warpgate::UrlLocator, PluginLocator};
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
                    .arg("https://github.com/moonrepo/tools/releases/latest/download/example_plugin.wasm");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path());

        assert_eq!(
            config.plugins.get("id").unwrap(),
            &PluginLocator::Url(Box::new(UrlLocator {
                url:
                    "https://github.com/moonrepo/tools/releases/latest/download/example_plugin.wasm"
                        .into()
            }))
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
                    .arg("https://github.com/moonrepo/tools/releases/latest/download/example_plugin.wasm")
                    .arg("--to")
                    .arg("global");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config.plugins.get("id").unwrap(),
            &PluginLocator::Url(Box::new(UrlLocator {
                url:
                    "https://github.com/moonrepo/tools/releases/latest/download/example_plugin.wasm"
                        .into()
            }))
        );
    }

    #[test]
    fn updates_user_file() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join(".home/.prototools");

        assert!(!config_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("plugin")
                    .arg("add")
                    .arg("id")
                    .arg("https://github.com/moonrepo/tools/releases/latest/download/example_plugin.wasm")
                    .arg("--to")
                    .arg("user");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".home"));

        assert_eq!(
            config.plugins.get("id").unwrap(),
            &PluginLocator::Url(Box::new(UrlLocator {
                url:
                    "https://github.com/moonrepo/tools/releases/latest/download/example_plugin.wasm"
                        .into()
            }))
        );
    }
}
