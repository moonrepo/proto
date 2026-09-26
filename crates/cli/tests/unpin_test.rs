use proto_core::test_utils::*;
use proto_core::{ProtoConfig, ToolContext, UnresolvedVersionSpec};
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeMap;
use std::fs;

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
            config.versions.get_or_insert_default().insert(
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
            config.versions.get_or_insert_default().insert(
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
                config.versions.get_or_insert_default().insert(
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

    mod tool_native {
        use super::*;

        #[test]
        fn removes_file() {
            let sandbox = create_empty_proto_sandbox();
            let version_file = sandbox.path().join(".protostar-version");

            fs::write(&version_file, "1.0.0").unwrap();

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("unpin").arg("protostar").arg("--tool-native");
                })
                .success();

            assert!(!version_file.exists());

            assert.stdout(predicate::str::contains("Removed protostar version 1.0.0"));
        }

        #[test]
        fn errors_if_tool_doesnt_support_it() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("unpin").arg("go").arg("--tool-native");
            });

            assert.failure().stderr(predicate::str::contains(
                "Go does not support unpinning from a native file",
            ));
        }

        #[test]
        fn bubbles_up_error_from_tool() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("unpin").arg("protostar").arg("--tool-native");
            });

            assert
                .failure()
                .stderr(predicate::str::contains("Version file does not exist."));
        }
    }
}

mod unpin_global {
    use super::*;

    #[test]
    fn removes_existing_pin() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path().join(".proto"), |config| {
            config.versions.get_or_insert_default().insert(
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

    mod tool_native {
        use super::*;

        #[test]
        fn removes_file() {
            let sandbox = create_empty_proto_sandbox();
            let version_file = sandbox.path().join(".proto/.protostar-version");

            fs::write(&version_file, "1.0.0").unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("unpin")
                        .arg("protostar")
                        .arg("--from")
                        .arg("global")
                        .arg("--tool-native");
                })
                .success();

            assert!(!version_file.exists());
        }
    }
}

mod unpin_user {
    use super::*;

    #[test]
    fn removes_existing_pin() {
        let sandbox = create_empty_proto_sandbox();

        ProtoConfig::update(sandbox.path().join(".home"), |config| {
            config.versions.get_or_insert_default().insert(
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

    mod tool_native {
        use super::*;

        #[test]
        fn removes_file() {
            let sandbox = create_empty_proto_sandbox();
            let version_file = sandbox.path().join(".home/.protostar-version");

            fs::write(&version_file, "1.0.0").unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("unpin")
                        .arg("protostar")
                        .arg("--from")
                        .arg("user")
                        .arg("--tool-native");
                })
                .success();

            assert!(!version_file.exists());
        }
    }
}

mod unpin_closest {
    use super::*;

    #[test]
    fn removes_from_closest_parent_config() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join("a/.prototools");

        sandbox.create_file(
            "a/.prototools",
            r#"protostar = "1.0.0"
moonstone = "2.0.0"
"#,
        );
        sandbox.create_file("a/b/c/file.txt", "");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin")
                    .arg("protostar")
                    .arg("--from")
                    .arg("closest")
                    .current_dir(sandbox.path().join("a/b/c"));
            })
            .success();

        assert!(!sandbox.path().join("a/b/c/.prototools").exists());
        assert_eq!(
            fs::read_to_string(config_file).unwrap(),
            "moonstone = \"2.0.0\"\n"
        );
    }

    #[test]
    fn prefers_local_config_over_parent() {
        let sandbox = create_empty_proto_sandbox();

        sandbox.create_file("a/.prototools", "protostar = \"1.0.0\"\n");
        sandbox.create_file("a/b/c/.prototools", "protostar = \"2.0.0\"\n");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin")
                    .arg("protostar")
                    .arg("--from")
                    .arg("closest")
                    .current_dir(sandbox.path().join("a/b/c"));
            })
            .success();

        assert_eq!(
            fs::read_to_string(sandbox.path().join("a/b/c/.prototools")).unwrap(),
            ""
        );
        assert_eq!(
            fs::read_to_string(sandbox.path().join("a/.prototools")).unwrap(),
            "protostar = \"1.0.0\"\n"
        );
    }

    #[test]
    fn ignores_global_config() {
        let sandbox = create_empty_proto_sandbox();

        sandbox.create_file(".proto/.prototools", "protostar = \"1.0.0\"\n");
        sandbox.create_file("a/b/c/file.txt", "");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin")
                    .arg("protostar")
                    .arg("--from")
                    .arg("closest")
                    .current_dir(sandbox.path().join("a/b/c"));
            })
            .failure();

        assert_eq!(
            fs::read_to_string(sandbox.path().join(".proto/.prototools")).unwrap(),
            "protostar = \"1.0.0\"\n"
        );
    }

    #[test]
    fn ignores_env_only_config() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join("a/.prototools");

        sandbox.create_file("a/.prototools", "protostar = \"1.0.0\"\n");
        sandbox.create_file("a/b/.prototools.prod", "moonstone = \"3.0.0\"\n");
        sandbox.create_file("a/b/c/file.txt", "");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin")
                    .arg("protostar")
                    .arg("--from")
                    .arg("closest")
                    .env("PROTO_ENV", "prod")
                    .current_dir(sandbox.path().join("a/b/c"));
            })
            .success();

        assert!(!sandbox.path().join("a/b/.prototools").exists());
        assert_eq!(fs::read_to_string(config_file).unwrap(), "");
    }

    #[test]
    fn is_the_default_location() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join("a/.prototools");

        sandbox.create_file("a/.prototools", "protostar = \"1.0.0\"\n");
        sandbox.create_file("a/b/c/file.txt", "");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin")
                    .arg("protostar")
                    .current_dir(sandbox.path().join("a/b/c"));
            })
            .success();

        assert_eq!(fs::read_to_string(config_file).unwrap(), "");
    }

    mod tool_native {
        use super::*;

        #[test]
        #[ignore = "WASM plugins can't access ancestors of the working directory"]
        fn removes_file_next_to_closest_config() {
            let sandbox = create_empty_proto_sandbox();
            let version_file = sandbox.path().join("a/.protostar-version");

            sandbox.create_file("a/.prototools", "");
            sandbox.create_file("a/.protostar-version", "1.0.0");
            sandbox.create_file("a/b/c/file.txt", "");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("unpin")
                        .arg("protostar")
                        .arg("--from")
                        .arg("closest")
                        .arg("--tool-native")
                        .current_dir(sandbox.path().join("a/b/c"));
                })
                .success();

            assert!(!version_file.exists());
        }
    }
}
