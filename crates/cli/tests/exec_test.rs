use proto_core::test_utils::*;
use starbase_sandbox::predicates::prelude::*;

mod exec {
    use super::*;

    #[test]
    fn errors_if_no_command() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("exec");
        });

        assert.failure().stderr(predicate::str::contains(
            "A command is required for execution.",
        ));
    }

    #[test]
    fn errors_for_invalid_context() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["exec", "foo bar", "--", "echo"]);
        });

        assert.failure().stderr(predicate::str::contains(
            "Invalid identifier format for `foo bar`.",
        ));
    }

    #[test]
    fn errors_for_invalid_spec() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["exec", "tool@a b c", "--", "echo"]);
        });

        assert.failure().stderr(predicate::str::contains(
            "Invalid version or requirement in tool specification a b c.",
        ));
    }

    // Stdout belongs to the tool being executed, so the reporter must render
    // to stderr, otherwise a failure corrupts the tool's output stream
    #[test]
    fn errors_use_reporter_output_in_agent_environments() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("exec")
                .env("CODEX_CI", "1")
                .env_remove("PROTO_REPORTER");
        });

        let stderr = assert.stderr();
        let records = stderr
            .lines()
            .filter(|line| line.starts_with('{'))
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();

        assert.failure().stdout(predicate::str::is_empty());
        assert!(records.iter().any(|record| {
            record.get("type").and_then(|value| value.as_str()) == Some("error")
                && record
                    .get("message")
                    .and_then(|value| value.as_str())
                    .is_some_and(|message| message.contains("A command is required for execution."))
        }));
    }

    #[test]
    fn can_execute_without_tools() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["exec", "--", "echo", "hello"]);
        });

        assert.success().stdout(predicate::str::contains("hello"));
    }

    #[cfg(unix)]
    #[test]
    fn restores_open_files_limit_for_command() {
        let sandbox = create_empty_proto_sandbox();

        // Start with a low soft limit, which proto raises for itself, but the
        // executed command should inherit the original limit. Without `-S`,
        // most shells lower the hard limit as well.
        let output = std::process::Command::new("sh")
            .current_dir(sandbox.path())
            .envs(&sandbox.settings.env)
            .arg("-c")
            .arg(r#"ulimit -S -n 256 && exec "$0" --log debug exec -- sh -c 'ulimit -S -n'"#)
            .arg(env!("CARGO_BIN_EXE_proto"))
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(output.status.success(), "{stderr}");
        assert_eq!(stdout.trim(), "256");
        assert!(
            stderr.contains("Raised the soft limit for open file descriptors"),
            "{stderr}"
        );
    }

    #[test]
    fn one_tool() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.args(["install", "node", "20"]);
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["exec", "node", "--", "node", "--version"]);
        });

        assert
            .success()
            .stdout(predicate::str::contains("v20.20.2"));
    }

    #[test]
    fn many_tools() {
        // Bare `install` loads all tools, so share the plugin store
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(
            ".prototools",
            r#"
node = "20"
bun = "1.2"
"#,
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args([
                "exec",
                "node",
                "bun",
                "--shell",
                "bash",
                "--raw",
                "--",
                "node --version && bun --version",
            ]);
        });

        assert
            .inner
            .stdout(predicate::str::contains("v20.20.2").and(predicate::str::contains("1.2.23")));
    }

    #[test]
    fn can_use_all_config_tools() {
        // Bare `install` loads all tools, so share the plugin store
        let sandbox = create_empty_proto_sandbox_with_shared_plugins();
        sandbox.create_file(
            ".prototools",
            r#"
node = "20"
bun = "1.2"
"#,
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args([
                "exec",
                "--tools-from-config",
                "--shell",
                "bash",
                "--raw",
                "--",
                "node --version && bun --version",
            ]);
        });

        assert
            .inner
            .stdout(predicate::str::contains("v20.20.2").and(predicate::str::contains("1.2.23")));
    }

    #[test]
    fn can_scope_by_version_part() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.args(["install", "node", "20.19.5"]);
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.args(["install", "node", "20.18.3"]);
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["exec", "node@20", "--", "node", "--version"]);
        });

        assert
            .success()
            .stdout(predicate::str::contains("v20.19.5"));

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["exec", "node@20.18", "--", "node", "--version"]);
        });

        assert
            .success()
            .stdout(predicate::str::contains("v20.18.3"));
    }
}
