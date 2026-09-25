mod trust {
    use proto_core::test_utils::*;
    use starbase_sandbox::predicates::prelude::*;
    use starbase_sandbox::{Sandbox, SandboxAssert};
    use std::path::Path;

    const CONFIG: &str = r#"
[env]
KEY = "value"

[shell.aliases]
gs = "git status"
"#;

    const WARNING: &str = "Review the config, then trust it with";

    // Configs in the sandbox are trusted by default, so reset the trusted
    // paths to test the trust model like it behaves for users
    fn run<'s>(
        sandbox: &'s Sandbox,
        dir: &Path,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> SandboxAssert<'s> {
        sandbox.run_bin(|cmd| {
            cmd.args(args)
                .current_dir(sandbox.path().join(dir))
                .env("PROTO_TRUSTED_PATHS", "");

            for (key, value) in env {
                cmd.env(key, value);
            }
        })
    }

    struct Activation {
        env: serde_json::Value,
        stderr: String,
    }

    fn activate_in(sandbox: &Sandbox, dir: &Path, env: &[(&str, &str)]) -> Activation {
        let assert = run(sandbox, dir, &["activate", "nu", "--reporter", "json"], env);
        let stdout = assert.stdout();
        let stderr = assert.stderr();

        assert.success();

        let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        Activation {
            env: output.get("env").unwrap().to_owned(),
            stderr,
        }
    }

    fn activate(sandbox: &Sandbox) -> Activation {
        activate_in(sandbox, Path::new(""), &[])
    }

    fn trust<'s>(sandbox: &'s Sandbox, args: &[&str]) -> SandboxAssert<'s> {
        let mut full_args = vec!["trust"];
        full_args.extend(args);

        run(sandbox, Path::new(""), &full_args, &[])
    }

    fn untrust<'s>(sandbox: &'s Sandbox, args: &[&str]) -> SandboxAssert<'s> {
        let mut full_args = vec!["untrust"];
        full_args.extend(args);

        run(sandbox, Path::new(""), &full_args, &[])
    }

    fn tracked_untrusted(activation: &Activation) -> &str {
        activation
            .env
            .get("_PROTO_ACTIVATED_UNTRUSTED")
            .unwrap()
            .as_str()
            .unwrap()
    }

    fn assert_untrusted(activation: &Activation) {
        assert!(activation.env.get("KEY").is_none());

        // Tracked by a hash of the path
        assert_eq!(tracked_untrusted(activation).len(), 16);
    }

    fn assert_trusted(activation: &Activation) {
        assert_eq!(activation.env.get("KEY").unwrap(), "value");
        assert!(activation.env.get("_PROTO_ACTIVATED_UNTRUSTED").is_none());
        assert!(!activation.stderr.contains(WARNING));
    }

    mod untrusted {
        use super::*;

        #[test]
        fn ignores_sensitive_settings_and_warns() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);

            let activation = activate(&sandbox);

            assert_untrusted(&activation);
            assert!(activation.stderr.contains(WARNING));
            assert!(activation.stderr.contains("env, shell.aliases"));
            assert!(activation.stderr.contains("proto trust"));
        }

        #[test]
        fn only_warns_once_per_shell_session() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);

            let first = activate(&sandbox);
            let tracked = tracked_untrusted(&first).to_owned();

            let second = activate_in(
                &sandbox,
                Path::new(""),
                &[("_PROTO_ACTIVATED_UNTRUSTED", &tracked)],
            );

            assert_untrusted(&second);
            assert!(!second.stderr.contains(WARNING));
        }

        #[test]
        fn warns_for_other_commands() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);

            run(&sandbox, Path::new(""), &["debug", "config"], &[])
                .success()
                .stderr(predicate::str::contains(WARNING));
        }

        #[test]
        fn does_not_warn_for_tool_commands() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", format!("protostar = \"1.0.0\"\n{CONFIG}"));

            // Tools are executed many times by scripts and editors
            let assert = run(&sandbox, Path::new(""), &["bin", "protostar"], &[]);

            assert!(!assert.stderr().contains(WARNING));
        }

        #[test]
        fn keeps_version_pins() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
node = "20.0.0"
proto = "0.40.0"

[env]
KEY = "value"

[tools.node.aliases]
work = "18"

[settings]
auto-install = true
lockfile = true
"#,
            );

            let assert = run(&sandbox, Path::new(""), &["debug", "config", "--json"], &[]);
            let stdout = assert.stdout();

            assert.success();

            let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            let config = output.get("config").unwrap();

            assert_eq!(config.get("node").unwrap(), "20.0.0");
            assert!(config.get("proto").is_none());
            assert!(config.get("env").is_none());
            assert!(config.pointer("/tools/node/aliases/work").is_some());
            assert_eq!(config.pointer("/settings/lockfile").unwrap(), true);
            assert_eq!(config.pointer("/settings/auto-install").unwrap(), false);
        }

        #[test]
        fn ignores_proto_pin_when_activating() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", "proto = \"0.40.0\"\n");

            let activation = activate(&sandbox);

            assert!(activation.env.get("PROTO_VERSION").unwrap().is_null());
            assert!(activation.env.get("PROTO_PROTO_VERSION").is_none());
        }

        #[test]
        fn errors_for_plugins_configured_in_untrusted_configs() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
[plugins.tools]
customtool = "file://./custom.wasm"
"#,
            );

            run(
                &sandbox,
                Path::new(""),
                &["install", "customtool", "1.0.0"],
                &[],
            )
            .failure()
            .stderr(predicate::str::contains("which has not been trusted"))
            .stderr(predicate::str::contains("proto trust"));
        }

        #[test]
        fn skips_tools_with_untrusted_plugins_when_activating() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
customtool = "1.0.0"

[plugins.tools]
customtool = "file://./custom.wasm"
"#,
            );

            let activation = activate(&sandbox);

            assert!(activation.stderr.contains(WARNING));
            assert!(activation.stderr.contains("plugins.tools"));
        }

        #[test]
        fn ignores_parent_configs() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);
            sandbox.create_file("child/.prototools", "");

            let activation = activate_in(&sandbox, Path::new("child"), &[]);

            assert_untrusted(&activation);
        }

        #[test]
        fn trusts_everything_within_trusted_paths() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);

            let activation = activate_in(
                &sandbox,
                Path::new(""),
                &[("PROTO_TRUSTED_PATHS", sandbox.path().to_str().unwrap())],
            );

            assert_trusted(&activation);
        }

        #[test]
        fn ignores_trusted_paths_set_by_a_previous_activation() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);

            let activation = activate_in(
                &sandbox,
                Path::new(""),
                &[
                    ("PROTO_TRUSTED_PATHS", sandbox.path().to_str().unwrap()),
                    ("_PROTO_ACTIVATED_ENV", "OTHER,PROTO_TRUSTED_PATHS"),
                ],
            );

            assert_untrusted(&activation);
        }

        #[test]
        fn removes_tracking_when_deactivating() {
            let sandbox = create_empty_proto_sandbox();

            let assert = run(
                &sandbox,
                Path::new(""),
                &["deactivate", "nu", "--reporter", "json"],
                &[("_PROTO_ACTIVATED_UNTRUSTED", "0123456789abcdef")],
            );
            let stdout = assert.stdout();

            assert.success();

            let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();

            assert!(
                output
                    .pointer("/env/_PROTO_ACTIVATED_UNTRUSTED")
                    .unwrap()
                    .is_null()
            );
        }
    }

    mod trusted {
        use super::*;

        #[test]
        fn applies_sensitive_settings() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);

            trust(&sandbox, &[])
                .success()
                .stdout(predicate::str::contains("Trusted directory"))
                .stdout(predicate::str::contains(".prototools"))
                .stdout(predicate::str::contains("shell.aliases"));

            assert_trusted(&activate(&sandbox));
        }

        #[test]
        fn keeps_trust_when_sensitive_settings_change() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);

            trust(&sandbox, &[]).success();

            sandbox.create_file(".prototools", CONFIG.replace("value", "changed"));

            let activation = activate(&sandbox);

            assert_eq!(activation.env.get("KEY").unwrap(), "changed");
            assert!(!activation.stderr.contains(WARNING));
        }

        #[test]
        fn trusts_nested_configs() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", "");
            sandbox.create_file("packages/child/.prototools", CONFIG);

            trust(&sandbox, &[]).success();

            assert_trusted(&activate_in(&sandbox, Path::new("packages/child"), &[]));
        }

        #[test]
        fn trusts_env_configs() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);
            sandbox.create_file(".prototools.prod", "[env]\nOTHER = \"value\"\n");

            trust(&sandbox, &[])
                .success()
                .stdout(predicate::str::contains(".prototools.prod"));

            let activation = activate_in(&sandbox, Path::new(""), &[("PROTO_ENV", "prod")]);

            assert_trusted(&activation);
            assert_eq!(activation.env.get("OTHER").unwrap(), "value");
        }

        #[test]
        fn can_trust_a_config_file() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file("child/.prototools", CONFIG);
            sandbox.create_file("child/.prototools.prod", "[env]\nOTHER = \"value\"\n");

            trust(&sandbox, &["child/.prototools"])
                .success()
                .stdout(predicate::str::contains("Trusted config"))
                .stdout(predicate::str::contains("shell.aliases"));

            // Only that file, not its siblings
            let activation = activate_in(&sandbox, Path::new("child"), &[("PROTO_ENV", "prod")]);

            assert_eq!(activation.env.get("KEY").unwrap(), "value");
            assert!(activation.env.get("OTHER").is_none());
            assert!(activation.stderr.contains(".prototools.prod"));
            assert_eq!(tracked_untrusted(&activation).len(), 16);
        }

        #[test]
        fn fails_when_config_file_does_not_exist() {
            let sandbox = create_empty_proto_sandbox();

            trust(&sandbox, &["child/.prototools"])
                .failure()
                .stderr(predicate::str::contains("does not exist"));
        }

        #[test]
        fn rejects_files_that_are_not_configs() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file("README.md", "");

            trust(&sandbox, &["README.md"])
                .failure()
                .stderr(predicate::str::contains("is not a .prototools config file"));
        }

        #[test]
        fn can_trust_a_directory_without_configs() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file("child/.gitkeep", "");

            trust(&sandbox, &["child"]).success();

            // Added later, for example by cloning a repository
            sandbox.create_file("child/.prototools", CONFIG);

            assert_trusted(&activate_in(&sandbox, Path::new("child"), &[]));
        }

        #[test]
        fn fails_when_directory_does_not_exist() {
            let sandbox = create_empty_proto_sandbox();

            trust(&sandbox, &["missing"])
                .failure()
                .stderr(predicate::str::contains("does not exist"));
        }

        #[test]
        fn can_untrust() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);

            trust(&sandbox, &[]).success();

            untrust(&sandbox, &[])
                .success()
                .stdout(predicate::str::contains("Untrusted directory"));

            assert_untrusted(&activate(&sandbox));

            untrust(&sandbox, &[])
                .success()
                .stdout(predicate::str::contains("was not trusted"));
        }

        #[test]
        fn untrust_reports_parent_directory_still_trusted() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file("child/.prototools", CONFIG);

            trust(&sandbox, &[]).success();

            untrust(&sandbox, &["child"])
                .success()
                .stdout(predicate::str::contains("was not trusted"))
                .stderr(predicate::str::contains("is trusted. Untrust it with"));

            assert_trusted(&activate_in(&sandbox, Path::new("child"), &[]));
        }

        #[test]
        fn can_untrust_a_config_file() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);

            trust(&sandbox, &[".prototools"]).success();

            untrust(&sandbox, &[".prototools"])
                .success()
                .stdout(predicate::str::contains("Untrusted config"));

            assert_untrusted(&activate(&sandbox));
        }

        #[test]
        fn untrust_file_reports_directory_still_trusted() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);

            trust(&sandbox, &[]).success();

            untrust(&sandbox, &[".prototools"])
                .success()
                .stdout(predicate::str::contains("Config"))
                .stdout(predicate::str::contains("was not trusted"))
                .stderr(predicate::str::contains("The config is still trusted"));

            assert_trusted(&activate(&sandbox));
        }
    }

    mod updates {
        use super::*;

        #[test]
        fn trusts_changes_made_by_proto_to_configs_without_sensitive_settings() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", "node = \"20\"\n");

            run(
                &sandbox,
                Path::new(""),
                &["plugin", "add", "customtool", "file://./custom.wasm"],
                &[],
            )
            .success();

            let activation = activate(&sandbox);

            assert!(activation.env.get("_PROTO_ACTIVATED_UNTRUSTED").is_none());
            assert!(!activation.stderr.contains(WARNING));
        }

        #[test]
        fn trusts_only_the_changed_config() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", "node = \"20\"\n");
            sandbox.create_file(".prototools.prod", "[env]\nOTHER = \"value\"\n");

            run(
                &sandbox,
                Path::new(""),
                &["plugin", "add", "customtool", "file://./custom.wasm"],
                &[],
            )
            .success();

            let activation = activate_in(&sandbox, Path::new(""), &[("PROTO_ENV", "prod")]);

            assert!(activation.env.get("OTHER").is_none());
            assert!(activation.stderr.contains(".prototools.prod"));
            assert!(!activation.stderr.contains(".prototools has not"));
        }

        #[test]
        fn keeps_trusted_configs_trusted_after_changes() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);

            trust(&sandbox, &[]).success();

            run(
                &sandbox,
                Path::new(""),
                &["plugin", "add", "customtool", "file://./custom.wasm"],
                &[],
            )
            .success();

            assert_trusted(&activate(&sandbox));
        }

        #[test]
        fn does_not_trust_changes_made_by_proto_to_untrusted_configs() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", CONFIG);

            run(
                &sandbox,
                Path::new(""),
                &["plugin", "add", "customtool", "file://./custom.wasm"],
                &[],
            )
            .success();

            assert_untrusted(&activate(&sandbox));
        }
    }
}
