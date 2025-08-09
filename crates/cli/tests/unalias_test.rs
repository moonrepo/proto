mod utils;

use proto_core::{Id, PartialProtoToolConfig, ProtoConfig, UnresolvedVersionSpec};
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeMap;
use utils::*;

mod unalias_local {
    use super::*;

    #[test]
    fn errors_unknown_tool() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("unalias").arg("unknown").arg("alias");
        });

        assert
            .inner
            .stderr(predicate::str::contains("unknown is not a built-in plugin"));
    }

    #[test]
    fn removes_existing_alias() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path(), |config| {
            config.tools.get_or_insert(Default::default()).insert(
                Id::raw("protostar"),
                PartialProtoToolConfig {
                    aliases: Some(BTreeMap::from_iter([(
                        "example".into(),
                        UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                    )])),
                    ..Default::default()
                },
            );
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unalias").arg("protostar").arg("example");
            })
            .success();

        let config = load_config(sandbox.path());

        assert!(!config.tools.contains_key("protostar"));
    }

    #[test]
    fn does_nothing_for_unknown_alias() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path(), |config| {
            config.tools.get_or_insert(Default::default()).insert(
                Id::raw("protostar"),
                PartialProtoToolConfig {
                    aliases: Some(BTreeMap::from_iter([(
                        "example".into(),
                        UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                    )])),
                    ..Default::default()
                },
            );
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unalias").arg("protostar").arg("unknown");
            })
            .failure();

        let config = load_config(sandbox.path());

        assert_eq!(
            config.tools.get("protostar").unwrap().aliases,
            BTreeMap::from_iter([(
                "example".into(),
                UnresolvedVersionSpec::parse("1.0.0").unwrap().into()
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
                config.tools.get_or_insert(Default::default()).insert(
                    Id::raw("act"),
                    PartialProtoToolConfig {
                        aliases: Some(BTreeMap::from_iter([(
                            "example".into(),
                            UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                        )])),
                        ..Default::default()
                    },
                );
            })
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("unalias").arg("asdf:act").arg("example");
                })
                .success();

            let config = load_config(sandbox.path());

            assert!(!config.tools.contains_key("act"));
        }
    }
}

mod unalias_global {
    use super::*;

    #[test]
    fn removes_existing_alias() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path().join(".proto"), |config| {
            config.tools.get_or_insert(Default::default()).insert(
                Id::raw("protostar"),
                PartialProtoToolConfig {
                    aliases: Some(BTreeMap::from_iter([(
                        "example".into(),
                        UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                    )])),
                    ..Default::default()
                },
            );
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unalias")
                    .arg("protostar")
                    .arg("example")
                    .arg("--from")
                    .arg("global");
            })
            .success();

        let config = load_config(sandbox.path().join(".proto"));

        assert!(!config.tools.contains_key("protostar"));
    }
}

mod unalias_user {
    use super::*;

    #[test]
    fn removes_existing_alias() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path().join(".home"), |config| {
            config.tools.get_or_insert(Default::default()).insert(
                Id::raw("protostar"),
                PartialProtoToolConfig {
                    aliases: Some(BTreeMap::from_iter([(
                        "example".into(),
                        UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                    )])),
                    ..Default::default()
                },
            );
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unalias")
                    .arg("protostar")
                    .arg("example")
                    .arg("--from")
                    .arg("user");
            })
            .success();

        let config = load_config(sandbox.path().join(".home"));

        assert!(!config.tools.contains_key("protostar"));
    }
}
