mod utils;

use proto_core::{Id, ProtoConfig, UnresolvedVersionSpec};
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeMap;
use utils::*;

mod unpin_local {
    use super::*;

    #[test]
    fn errors_unknown_tool() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("unpin").arg("unknown");
        });

        assert
            .inner
            .stderr(predicate::str::contains("unknown is not a built-in tool"));
    }

    #[test]
    fn removes_existing_pin() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path(), |config| {
            config
                .versions
                .get_or_insert(Default::default())
                .insert(Id::raw("node"), UnresolvedVersionSpec::Canary);
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin").arg("node");
            })
            .success();

        let config = load_config(sandbox.path());

        assert!(!config.versions.contains_key("node"));
    }

    #[test]
    fn does_nothing_for_unknown_pin() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path(), |config| {
            config
                .versions
                .get_or_insert(Default::default())
                .insert(Id::raw("bun"), UnresolvedVersionSpec::Canary);
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin").arg("node");
            })
            .failure();

        let config = load_config(sandbox.path());

        assert_eq!(
            config.versions,
            BTreeMap::from_iter([(Id::raw("bun"), UnresolvedVersionSpec::Canary)])
        );
    }
}

mod unpin_global {
    use super::*;

    #[test]
    fn removes_existing_pin() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path().join(".proto"), |config| {
            config
                .versions
                .get_or_insert(Default::default())
                .insert(Id::raw("node"), UnresolvedVersionSpec::Canary);
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin").arg("node").arg("--global");
            })
            .success();

        let config = load_config(sandbox.path().join(".proto"));

        assert!(!config.versions.contains_key("node"));
    }
}

mod unpin_user {
    use super::*;

    #[test]
    fn removes_existing_pin() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path().join(".home"), |config| {
            config
                .versions
                .get_or_insert(Default::default())
                .insert(Id::raw("node"), UnresolvedVersionSpec::Canary);
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin").arg("node").arg("--from").arg("user");
            })
            .success();

        let config = load_config(sandbox.path().join(".home"));

        assert!(!config.versions.contains_key("node"));
    }
}
