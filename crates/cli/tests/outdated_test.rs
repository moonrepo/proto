use proto_core::test_utils::*;
use starbase_sandbox::assert_debug_snapshot;
use starbase_sandbox::predicates::prelude::*;
use std::fs;

mod outdated {
    use super::*;

    #[test]
    fn errors_when_nothing_configured() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated");
        });

        assert
            .inner
            .stderr(predicate::str::contains("No tools have been configured"));
    }

    #[test]
    fn reports_all_non_global_configs() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".proto/.prototools", r#"moonbase = "*""#);
        sandbox.create_file("a/.prototools", r#"protostar = "*""#);
        sandbox.create_file("a/b/.prototools", r#"moonstone = "*""#);
        sandbox.create_file("a/b/c/.prototools", r#"protoform = "*""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated")
                .current_dir(sandbox.path().join("a/b/c"));
        });

        let output = assert.output();

        assert!(predicate::str::contains("protostar").eval(&output));
        assert!(predicate::str::contains("moonstone").eval(&output));
        assert!(predicate::str::contains("protoform").eval(&output));
        assert!(predicate::str::contains("moonbase").not().eval(&output));
    }

    #[test]
    fn only_includes_local_config() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".proto/.prototools", r#"moonbase = "*""#);
        sandbox.create_file("a/.prototools", r#"protostar = "*""#);
        sandbox.create_file("a/b/.prototools", r#"moonstone = "*""#);
        sandbox.create_file("a/b/c/.prototools", r#"protoform = "*""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated")
                .arg("--config-mode")
                .arg("local")
                .current_dir(sandbox.path().join("a/b/c"));
        });

        let output = assert.output();

        assert!(predicate::str::contains("protostar").not().eval(&output));
        assert!(predicate::str::contains("moonstone").not().eval(&output));
        assert!(predicate::str::contains("protoform").eval(&output));
        assert!(predicate::str::contains("moonbase").not().eval(&output));
    }

    #[test]
    fn can_include_global_config() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".proto/.prototools", r#"moonbase = "*""#);
        sandbox.create_file("a/.prototools", r#"protostar = "*""#);
        sandbox.create_file("a/b/.prototools", r#"moonstone = "*""#);
        sandbox.create_file("a/b/c/.prototools", r#"protoform = "*""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated")
                .arg("--config-mode")
                .arg("upwards-global")
                .current_dir(sandbox.path().join("a/b/c"));
        });

        let output = assert.output();

        assert!(predicate::str::contains("protostar").eval(&output));
        assert!(predicate::str::contains("moonstone").eval(&output));
        assert!(predicate::str::contains("protoform").eval(&output));
        assert!(predicate::str::contains("moonbase").eval(&output));
    }

    fn get_versions(sandbox: &ProtoSandbox, tool: &str) -> (String, String, String) {
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated").arg("--json");
        });
        let stdout = assert.stdout();
        assert.success();

        let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let item = &output[tool];
        let get = |key: &str| item[key].as_str().unwrap().to_owned();

        (
            get("current_version"),
            get("newest_version"),
            get("latest_version"),
        )
    }

    // https://github.com/moonrepo/proto/issues/1120
    #[test]
    fn newest_ignores_installed_versions_for_requirement() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".prototools", r#"protostar = "^4.2.0""#);
        sandbox.create_file(
            ".proto/tools/protostar/manifest.json",
            r#"{ "installed_versions": ["4.2.0"] }"#,
        );

        assert_eq!(
            get_versions(&sandbox, "protostar"),
            ("4.2.0".into(), "4.10.15".into(), "5.10.15".into())
        );
    }

    #[test]
    fn newest_ignores_installed_versions_for_explicit_version() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".prototools", r#"protostar = "4.2.0""#);
        sandbox.create_file(
            ".proto/tools/protostar/manifest.json",
            r#"{ "installed_versions": ["4.2.0"] }"#,
        );

        assert_eq!(
            get_versions(&sandbox, "protostar"),
            ("4.2.0".into(), "4.10.15".into(), "5.10.15".into())
        );
    }

    #[test]
    fn newest_resolves_config_aliases() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(
            ".prototools",
            r#"protostar = "work"

[tools.protostar.aliases]
work = "^4.2.0"
"#,
        );
        sandbox.create_file(
            ".proto/tools/protostar/manifest.json",
            r#"{ "installed_versions": ["4.2.0"] }"#,
        );

        assert_eq!(
            get_versions(&sandbox, "protostar"),
            ("4.2.0".into(), "4.10.15".into(), "5.10.15".into())
        );
    }

    #[test]
    fn global_doesnt_overwrite_local() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".proto/.prototools", r#"protostar = "1""#);
        sandbox.create_file("a/.prototools", r#"protostar = "2""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated")
                .arg("--config-mode")
                .arg("upwards-global")
                .current_dir(sandbox.path().join("a"));
        });

        let output = assert.output();

        assert!(predicate::str::contains("protostar").eval(&output));
        assert!(predicate::str::contains("2").eval(&output));
    }

    #[test]
    fn updates_each_file_respectively() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".proto/.prototools", r#"moonbase = "1""#);
        sandbox.create_file("a/.prototools", r#"protostar = "2""#);
        sandbox.create_file("a/b/.prototools", r#"moonstone = "3""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("outdated")
                    .arg("--update")
                    .arg("--config-mode")
                    .arg("upwards-global")
                    .arg("--yes")
                    .current_dir(sandbox.path().join("a/b"));
            })
            .success();

        assert_debug_snapshot!(vec![
            fs::read_to_string(sandbox.path().join(".proto/.prototools")).unwrap(),
            fs::read_to_string(sandbox.path().join("a/.prototools")).unwrap(),
            fs::read_to_string(sandbox.path().join("a/b/.prototools")).unwrap(),
        ]);
    }

    #[test]
    fn can_update_with_latest_version() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".proto/.prototools", r#"moonbase = "1.0.0""#);
        sandbox.create_file("a/.prototools", r#"protostar = "2.0.0""#);
        sandbox.create_file("a/b/.prototools", r#"moonstone = "3.0.0""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("outdated")
                    .arg("--update")
                    .arg("--config-mode")
                    .arg("upwards-global")
                    .arg("--latest")
                    .arg("--yes")
                    .current_dir(sandbox.path().join("a/b"));
            })
            .success();

        assert!(
            !fs::read_to_string(sandbox.path().join(".proto/.prototools"))
                .unwrap()
                .contains("1.0.0")
        );
        assert!(
            !fs::read_to_string(sandbox.path().join("a/.prototools"))
                .unwrap()
                .contains("2.0.0")
        );
        assert!(
            !fs::read_to_string(sandbox.path().join("a/b/.prototools"))
                .unwrap()
                .contains("3.0.0")
        );
    }

    #[test]
    fn updates_when_using_json_format() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".prototools", r#"moonbase = "1.0.0""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("outdated")
                    .arg("--update")
                    .arg("--latest")
                    .arg("--yes")
                    .arg("--json");
            })
            .success();

        assert!(
            !fs::read_to_string(sandbox.path().join(".prototools"))
                .unwrap()
                .contains("1.0.0")
        );
    }

    #[test]
    fn updates_when_using_ndjson_format() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".prototools", r#"moonbase = "1.0.0""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("outdated")
                    .arg("--update")
                    .arg("--latest")
                    .arg("--yes")
                    .arg("--reporter")
                    .arg("ndjson");
            })
            .success();

        assert!(
            !fs::read_to_string(sandbox.path().join(".prototools"))
                .unwrap()
                .contains("1.0.0")
        );
    }

    // Prompts can't be answered without a terminal, so the `--update`
    // flag itself is the confirmation
    #[test]
    fn updates_without_yes_when_not_a_tty() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".prototools", r#"moonbase = "1.0.0""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("outdated").arg("--update").arg("--latest");
            })
            .success();

        assert!(
            !fs::read_to_string(sandbox.path().join(".prototools"))
                .unwrap()
                .contains("1.0.0")
        );
    }

    #[test]
    fn warns_when_nothing_was_updated() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".prototools", r#"moonbase = "latest""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated").arg("--update").arg("--yes");
        });

        let output = assert.output();

        assert!(predicate::str::contains("No versions were updated!").eval(&output));
    }

    #[test]
    fn doesnt_overwrite_aliases() {
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(".prototools", r#"moonbase = "latest""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("outdated").arg("--update").arg("--yes");
            })
            .success();

        assert_debug_snapshot!(vec![
            fs::read_to_string(sandbox.path().join(".prototools")).unwrap(),
        ]);
    }
}
