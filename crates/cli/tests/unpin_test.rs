mod utils;

use proto_core::{ProtoConfig, ToolContext, UnresolvedVersionSpec};
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
            .stderr(predicate::str::contains("unknown is not a built-in plugin"));
    }

    #[test]
    fn removes_existing_pin() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path(), |config| {
            config.versions.get_or_insert(Default::default()).insert(
                ToolContext::parse("protostar").unwrap(),
                UnresolvedVersionSpec::Canary.into(),
            );
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin").arg("protostar");
            })
            .success();

        let config = load_config(sandbox.path());

        assert!(
            !config
                .versions
                .contains_key(&ToolContext::parse("protostar").unwrap())
        );
    }

    #[test]
    fn does_nothing_for_unknown_pin() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path(), |config| {
            config.versions.get_or_insert(Default::default()).insert(
                ToolContext::parse("moonstone").unwrap(),
                UnresolvedVersionSpec::Canary.into(),
            );
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin").arg("protostar");
            })
            .failure();

        let config = load_config(sandbox.path());

        assert_eq!(
            config.versions,
            BTreeMap::from_iter([(
                ToolContext::parse("moonstone").unwrap(),
                UnresolvedVersionSpec::Canary.into()
            )])
        );
    }

    // Windows doesn't support asdf
    #[cfg(unix)]
    mod backend {
        use super::*;

        #[test]
        fn can_remove() {
            let sandbox = create_empty_proto_sandbox();

            ProtoConfig::update(sandbox.path(), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    ToolContext::parse("asdf:act").unwrap(),
                    UnresolvedVersionSpec::Canary.into(),
                );
            })
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("unpin").arg("asdf:act");
                })
                .success();

            let config = load_config(sandbox.path());

            assert!(
                !config
                    .versions
                    .contains_key(&ToolContext::parse("asdf:act").unwrap())
            );
        }
    }
}

mod unpin_global {
    use super::*;

    #[test]
    fn removes_existing_pin() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path().join(".proto"), |config| {
            config.versions.get_or_insert(Default::default()).insert(
                ToolContext::parse("protostar").unwrap(),
                UnresolvedVersionSpec::Canary.into(),
            );
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin")
                    .arg("protostar")
                    .arg("--from")
                    .arg("global");
            })
            .success();

        let config = load_config(sandbox.path().join(".proto"));

        assert!(
            !config
                .versions
                .contains_key(&ToolContext::parse("protostar").unwrap())
        );
    }
}

mod unpin_user {
    use super::*;

    #[test]
    fn removes_existing_pin() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path().join(".home"), |config| {
            config.versions.get_or_insert(Default::default()).insert(
                ToolContext::parse("protostar").unwrap(),
                UnresolvedVersionSpec::Canary.into(),
            );
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin").arg("protostar").arg("--from").arg("user");
            })
            .success();

        let config = load_config(sandbox.path().join(".home"));

        assert!(
            !config
                .versions
                .contains_key(&ToolContext::parse("protostar").unwrap())
        );
    }
}
