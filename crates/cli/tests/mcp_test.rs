mod mcp {
    use proto_core::test_utils::*;

    #[test]
    fn stdout_only_carries_jsonrpc_in_agent_environments() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("mcp")
                .env("CODEX_CI", "1")
                .env_remove("PROTO_REPORTER")
                .write_stdin(
                    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#
                        .to_owned()
                        + "\n",
                );
        });
        let stdout = assert.stdout();

        assert!(stdout.contains("\"jsonrpc\":\"2.0\""));

        for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
            let record: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|error| {
                panic!("stdout line is not valid JSON-RPC: {line}: {error}")
            });

            assert_eq!(
                record.get("jsonrpc").and_then(|value| value.as_str()),
                Some("2.0"),
                "stdout line is not a JSON-RPC record: {line}",
            );
        }

        assert.success();
    }
}
