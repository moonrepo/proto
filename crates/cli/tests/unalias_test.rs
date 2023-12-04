mod utils;

use proto_core::{Id, PartialProtoToolConfig, ProtoConfig, UnresolvedVersionSpec};
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeMap;
use utils::*;

mod unalias_local {
    use super::*;

    #[test]
    fn errors_unknown_tool() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("unalias").arg("unknown").arg("alias").assert();

        assert.stderr(predicate::str::contains("unknown is not a built-in tool"));
    }

    #[test]
    fn removes_existing_alias() {
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
        cmd.arg("unalias")
            .arg("node")
            .arg("example")
            .assert()
            .success();

        let config = load_config(sandbox.path());

        assert!(config.tools.get("node").unwrap().aliases.is_empty());
    }

    #[test]
    fn does_nothing_for_unknown_alias() {
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
        cmd.arg("unalias")
            .arg("node")
            .arg("unknown")
            .assert()
            .success();

        let config = load_config(sandbox.path());

        assert_eq!(
            config.tools.get("node").unwrap().aliases,
            BTreeMap::from_iter([(
                "example".into(),
                UnresolvedVersionSpec::parse("19.0.0").unwrap()
            )])
        );
    }
}

mod unalias_global {
    use super::*;

    #[test]
    fn removes_existing_alias() {
        let sandbox = create_empty_sandbox();

        ProtoConfig::update(sandbox.path().join(".proto"), |config| {
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
        cmd.arg("unalias")
            .arg("node")
            .arg("example")
            .arg("--global")
            .assert()
            .success();

        let config = load_config(sandbox.path().join(".proto"));

        assert!(config.tools.get("node").unwrap().aliases.is_empty());
    }
}
