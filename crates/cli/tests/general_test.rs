use proto_core::test_utils::*;

mod general {
    use super::*;

    #[test]
    fn can_write_to_a_log_file() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("debug")
                    .arg("config")
                    .arg("--log-file")
                    .arg("./proto.log")
                    .arg("--log")
                    .arg("trace");
            })
            .success();

        assert!(sandbox.path().join("proto.log").exists());
    }

    #[test]
    fn can_write_to_a_log_file_with_env_var() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("debug")
                    .arg("config")
                    .arg("--log")
                    .arg("trace")
                    .env("PROTO_LOG_FILE", "./proto.log");
            })
            .success();

        assert!(sandbox.path().join("proto.log").exists());
    }

    #[test]
    fn json_flag_overrides_reporter_env() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("--json")
                .arg("mcp")
                .arg("--info")
                .env("PROTO_REPORTER", "ndjson")
                .env_remove("PROTO_TEST");
        });
        let stdout = assert.stdout();
        assert.success();
        let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        assert!(output.get("info").is_some());
    }

    #[test]
    fn reporter_flag_overrides_json_env() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("--reporter")
                .arg("ndjson")
                .arg("mcp")
                .arg("--info")
                .env("PROTO_JSON", "true")
                .env_remove("PROTO_TEST");
        });
        let stdout = assert.stdout();
        assert.success();
        let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        assert_eq!(
            output.get("type").and_then(|value| value.as_str()),
            Some("data")
        );
    }

    #[test]
    fn reporter_wins_legacy_json_alias() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("--json")
                .arg("--reporter")
                .arg("ndjson")
                .arg("mcp")
                .arg("--info")
                .env_remove("PROTO_TEST");
        });
        let stdout = assert.stdout();
        assert.success();
        let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        assert_eq!(
            output.get("type").and_then(|value| value.as_str()),
            Some("data")
        );
    }

    #[test]
    fn json_env_selects_json_reporter() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("mcp")
                .arg("--info")
                .env("PROTO_JSON", "true")
                .env_remove("PROTO_REPORTER")
                .env_remove("PROTO_TEST");
        });
        let stdout = assert.stdout();
        assert.success();
        let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        assert!(output.get("info").is_some());
    }
}
