mod completions {
    use proto_core::test_utils::*;
    use starbase_sandbox::predicates::prelude::*;

    #[test]
    fn prints_only_completion_code_in_agent_environments() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("completions")
                .arg("--shell")
                .arg("zsh")
                .env("CODEX_CI", "1")
                .env_remove("PROTO_REPORTER");
        });
        assert.success().stdout(
            predicate::str::contains("#compdef proto")
                .and(predicate::str::contains("{\"type\":").not()),
        );
    }

    #[test]
    fn unsupported_shell_notice_goes_to_stderr() {
        let sandbox = create_empty_proto_sandbox();

        // Even an explicit structured reporter cannot take over this stdout:
        // the documented usage redirects it into a completion file
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("completions")
                .arg("--shell")
                .arg("ion")
                .arg("--reporter")
                .arg("ndjson");
        });
        let stderr = assert.stderr();
        let records = stderr
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();

        assert.failure().stdout(predicate::str::is_empty());
        assert!(records.iter().any(|record| {
            record.get("type").and_then(|value| value.as_str()) == Some("notice")
                && record
                    .get("messages")
                    .and_then(|value| value.as_array())
                    .is_some_and(|messages| {
                        messages.iter().any(|message| {
                            message.as_str().is_some_and(|message| {
                                message.contains("does not currently support completions")
                            })
                        })
                    })
        }));
    }
}
