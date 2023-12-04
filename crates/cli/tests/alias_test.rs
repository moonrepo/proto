mod utils;

use proto_core::{Id, PartialProtoToolConfig, ProtoConfig, UnresolvedVersionSpec};
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeMap;
use utils::*;

mod alias_local {
    use super::*;

    #[test]
    fn errors_unknown_tool() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("alias")
            .arg("unknown")
            .arg("alias")
            .arg("1.2.3")
            .assert();

        assert.stderr(predicate::str::contains("unknown is not a built-in tool"));
    }

    #[test]
    fn updates_config_file() {
        let sandbox = create_empty_sandbox();
        let config_file = sandbox.path().join(".prototools");

        assert!(!config_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("alias")
            .arg("node")
            .arg("example")
            .arg("19.0.0")
            .current_dir(sandbox.path())
            .assert()
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path());

        assert_eq!(
            config.tools.get("node").unwrap().aliases,
            BTreeMap::from_iter([(
                "example".into(),
                UnresolvedVersionSpec::parse("19.0.0").unwrap()
            )])
        );
    }

    #[test]
    fn can_overwrite_existing_alias() {
        let sandbox = create_empty_sandbox();

        ProtoConfig::update(sandbox.path(), |config| {
            config.tools.get_or_insert(Default::default()).insert(
                Id::raw("node"),
                PartialProtoToolConfig {
                    aliases: Some(BTreeMap::from_iter([(
                        "example".into(),
                        UnresolvedVersionSpec::parse("19.0.0").unwrap(),
                    )])),
                    ..Default::default()
                },
            );
        })
        .unwrap();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("alias")
            .arg("node")
            .arg("example")
            .arg("20.0.0")
            .assert()
            .success();

        let config = load_config(sandbox.path());

        assert_eq!(
            config.tools.get("node").unwrap().aliases,
            BTreeMap::from_iter([(
                "example".into(),
                UnresolvedVersionSpec::parse("20.0.0").unwrap()
            )])
        );
    }

    #[test]
    fn errors_when_using_version() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("alias")
            .arg("node")
            .arg("1.2.3")
            .arg("4.5.6")
            .assert();

        assert.stderr(predicate::str::contains(
            "Invalid alias name 1.2.3. Use alphanumeric words instead.",
        ));
    }

    #[test]
    fn errors_when_aliasing_self() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("alias")
            .arg("node")
            .arg("example")
            .arg("example")
            .assert();

        assert.stderr(predicate::str::contains("Cannot map an alias to itself."));
    }
}

mod alias_global {
    use super::*;

    #[test]
    fn updates_config_file() {
        let sandbox = create_empty_sandbox();
        let config_file = sandbox.path().join(".proto/.prototools");

        assert!(!config_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("alias")
            .arg("node")
            .arg("example")
            .arg("19.0.0")
            .arg("--global")
            .assert()
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config.tools.get("node").unwrap().aliases,
            BTreeMap::from_iter([(
                "example".into(),
                UnresolvedVersionSpec::parse("19.0.0").unwrap()
            )])
        );
    }
}
