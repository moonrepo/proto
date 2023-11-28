mod utils;

use proto_core::{Id, UnresolvedVersionSpec, UserConfig, UserToolConfig};
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeMap;
use utils::*;

mod unalias {
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

        let mut config = UserConfig::load_from(sandbox.path()).unwrap();
        config.tools.insert(
            Id::raw("node"),
            UserToolConfig {
                aliases: BTreeMap::from_iter([(
                    "example".into(),
                    UnresolvedVersionSpec::parse("19.0.0").unwrap(),
                )]),
                ..Default::default()
            },
        );
        config.save().unwrap();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("unalias")
            .arg("node")
            .arg("example")
            .assert()
            .success();

        let config = UserConfig::load_from(sandbox.path()).unwrap();

        assert!(config.tools.get("node").unwrap().aliases.is_empty());
    }

    #[test]
    fn does_nothing_for_unknown_alias() {
        let sandbox = create_empty_sandbox();

        let mut config = UserConfig::load_from(sandbox.path()).unwrap();
        config.tools.insert(
            Id::raw("node"),
            UserToolConfig {
                aliases: BTreeMap::from_iter([(
                    "example".into(),
                    UnresolvedVersionSpec::parse("19.0.0").unwrap(),
                )]),
                ..Default::default()
            },
        );
        config.save().unwrap();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("unalias")
            .arg("node")
            .arg("unknown")
            .assert()
            .success();

        let config = UserConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(
            config.tools.get("node").unwrap().aliases,
            BTreeMap::from_iter([(
                "example".into(),
                UnresolvedVersionSpec::parse("19.0.0").unwrap()
            )])
        );
    }
}
