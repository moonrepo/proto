mod completions {
    use proto_core::test_utils::*;
    use starbase_sandbox::predicates::prelude::*;

    // Use the real reporter with AI agent detection forced on.

    #[test]
    fn prints_only_completion_code_in_agent_environments() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("completions")
                .arg("--shell")
                .arg("zsh")
                .env("CODEX_CI", "1")
                .env_remove("PROTO_TEST");
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
                .arg("ndjson")
                .env_remove("PROTO_TEST");
        });

        assert
            .failure()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains(
                "does not currently support completions",
            ));
    }
}
