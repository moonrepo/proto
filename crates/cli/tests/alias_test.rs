mod utils;

use proto_core::{
    Backend, Id, PartialProtoToolConfig, ProtoConfig, ToolSpec, UnresolvedVersionSpec,
};
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeMap;
use utils::*;

mod alias_local {
    use super::*;

    #[test]
    fn errors_unknown_tool() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("alias").arg("unknown").arg("alias").arg("1.2.3");
        });

        assert
            .inner
            .stderr(predicate::str::contains("unknown is not a built-in plugin"));
    }

    #[test]
    fn updates_config_file() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join(".prototools");

        assert!(!config_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("alias")
                    .arg("node")
                    .arg("example")
                    .arg("19.0.0")
                    .current_dir(sandbox.path());
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path());

        assert_eq!(
            config.tools.get("node").unwrap().aliases,
            BTreeMap::from_iter([(
                "example".into(),
                UnresolvedVersionSpec::parse("19.0.0").unwrap().into()
            )])
        );
    }

    #[test]
    fn can_overwrite_existing_alias() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path(), |config| {
            config.tools.get_or_insert(Default::default()).insert(
                Id::raw("node"),
                PartialProtoToolConfig {
                    aliases: Some(BTreeMap::from_iter([(
                        "example".into(),
                        UnresolvedVersionSpec::parse("19.0.0").unwrap().into(),
                    )])),
                    ..Default::default()
                },
            );
        })
        .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("alias").arg("node").arg("example").arg("20.0.0");
            })
            .success();

        let config = load_config(sandbox.path());

        assert_eq!(
            config.tools.get("node").unwrap().aliases,
            BTreeMap::from_iter([(
                "example".into(),
                UnresolvedVersionSpec::parse("20.0.0").unwrap().into()
            )])
        );
    }

    #[test]
    fn errors_when_using_version() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("alias").arg("node").arg("1.2.3").arg("4.5.6");
        });

        assert.inner.stderr(predicate::str::contains(
            "Invalid alias name 1.2.3. Use alpha-numeric words instead.",
        ));
    }

    #[test]
    fn errors_when_aliasing_self() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("alias").arg("node").arg("example").arg("example");
        });

        assert
            .inner
            .stderr(predicate::str::contains("Cannot map an alias to itself."));
    }

    #[cfg(not(windows))]
    mod backend {
        use super::*;

        #[test]
        fn can_set_asdf() {
            let sandbox = create_empty_proto_sandbox();
            let config_file = sandbox.path().join(".prototools");

            assert!(!config_file.exists());

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("alias")
                        .arg("act")
                        .arg("example")
                        .arg("asdf:0.2")
                        .current_dir(sandbox.path());
                })
                .success();

            assert!(config_file.exists());

            let config = load_config(sandbox.path());

            assert_eq!(
                config.tools.get("act").unwrap().aliases,
                BTreeMap::from_iter([(
                    "example".into(),
                    ToolSpec {
                        backend: Some(Backend::Asdf),
                        req: UnresolvedVersionSpec::parse("0.2").unwrap(),
                        res: None,
                    }
                )])
            );
        }
    }
}

mod alias_global {
    use super::*;

    #[test]
    fn updates_config_file() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join(".proto/.prototools");

        assert!(!config_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("alias")
                    .arg("node")
                    .arg("example")
                    .arg("19.0.0")
                    .arg("--to")
                    .arg("global");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config.tools.get("node").unwrap().aliases,
            BTreeMap::from_iter([(
                "example".into(),
                UnresolvedVersionSpec::parse("19.0.0").unwrap().into()
            )])
        );
    }
}

mod alias_user {
    use super::*;

    #[test]
    fn updates_config_file() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join(".home/.prototools");

        assert!(!config_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("alias")
                    .arg("node")
                    .arg("example")
                    .arg("19.0.0")
                    .arg("--to")
                    .arg("user");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".home"));

        assert_eq!(
            config.tools.get("node").unwrap().aliases,
            BTreeMap::from_iter([(
                "example".into(),
                UnresolvedVersionSpec::parse("19.0.0").unwrap().into()
            )])
        );
    }
}
