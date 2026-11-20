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
    fn ndjson_reporter_overrides_json_flag() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("--json")
                .arg("mcp")
                .arg("--info")
                .env("PROTO_REPORTER", "ndjson")
                .env_remove("CODEX_CI")
                .env_remove("CODEX_SANDBOX")
                .env_remove("CODEX_THREAD_ID");
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
    fn json_flag_overrides_text_reporter_env() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("--json")
                .arg("mcp")
                .arg("--info")
                .env("PROTO_REPORTER", "text")
                .env_remove("CODEX_CI")
                .env_remove("CODEX_SANDBOX")
                .env_remove("CODEX_THREAD_ID");
        });
        let stdout = assert.stdout();
        assert.success();
        let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        assert!(output.get("info").is_some());
    }

    #[test]
    fn json_env_selects_json_reporter() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("mcp")
                .arg("--info")
                .env("CODEX_CI", "1")
                .env("PROTO_JSON", "true")
                .env_remove("PROTO_REPORTER");
        });
        let stdout = assert.stdout();
        assert.success();
        let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        assert!(output.get("info").is_some());
    }

    mod agent_reporter {
        use super::*;

        const NDJSON_NOTICE: &str = "Detected an AI agent environment";

        // Runs proto with a terminal attached, like a person typing a command
        #[cfg(unix)]
        fn run_in_terminal(sandbox: &ProtoSandbox, envs: &[(&str, &str)]) -> String {
            use std::process::{Command, Stdio};

            let proto = env!("CARGO_BIN_EXE_proto");
            let mut cmd = Command::new("script");

            if cfg!(target_os = "macos") {
                cmd.args(["-q", "/dev/null", proto, "status"]);
            } else {
                cmd.args(["-q", "-e", "-c", &format!("'{proto}' status"), "/dev/null"]);
            }

            cmd.current_dir(sandbox.path())
                .envs(&sandbox.settings.env)
                .env_remove("PROTO_REPORTER")
                .envs(envs.iter().copied())
                .stdin(Stdio::null());

            String::from_utf8_lossy(&cmd.output().unwrap().stdout).into_owned()
        }

        #[test]
        fn defaults_to_ndjson_when_an_agent_captures_output_in_cursor() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("status")
                    .env("CURSOR_TRACE_ID", "abc")
                    .env_remove("PROTO_REPORTER");
            });

            assert!(assert.stdout().contains(NDJSON_NOTICE));
        }

        // Cursor's integrated terminal sets `CURSOR_TRACE_ID` for every command
        #[cfg(unix)]
        #[test]
        fn defaults_to_text_for_a_person_in_cursors_terminal() {
            let sandbox = create_empty_proto_sandbox();

            let output = run_in_terminal(&sandbox, &[("CURSOR_TRACE_ID", "abc")]);

            assert!(output.contains("No tools have been configured"), "{output}");
            assert!(!output.contains(NDJSON_NOTICE), "{output}");
            assert!(!output.contains(r#"{"type":"#), "{output}");
        }

        #[cfg(unix)]
        #[test]
        fn defaults_to_ndjson_for_cursors_agent_in_a_terminal() {
            let sandbox = create_empty_proto_sandbox();

            let output = run_in_terminal(
                &sandbox,
                &[("CURSOR_TRACE_ID", "abc"), ("CURSOR_AGENT", "1")],
            );

            assert!(output.contains(NDJSON_NOTICE), "{output}");
        }

        // Agents in Cursor's terminal inherit `CURSOR_TRACE_ID`
        #[cfg(unix)]
        #[test]
        fn defaults_to_ndjson_for_other_agents_in_cursors_terminal() {
            let sandbox = create_empty_proto_sandbox();

            let output =
                run_in_terminal(&sandbox, &[("CURSOR_TRACE_ID", "abc"), ("CLAUDECODE", "1")]);

            assert!(output.contains(NDJSON_NOTICE), "{output}");
        }
    }

    mod log_level {
        use super::*;

        // Logged when loading a built-in plugin in debug builds, which
        // makes it the only `info` log that all of these commands reach
        const INFO_LOG: &str = "Using a locally built WASM file";

        const TOOL_COMMANDS: &[&[&str]] = &[
            &["run", "protostar"],
            &["exec", "protostar", "--", "echo", "hello"],
            &["bin", "protostar"],
        ];

        fn get_stderr(args: &[&str], level: Option<&str>) -> String {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", "protostar = \"1.0.0\"\n");

            sandbox
                .run_bin(|cmd| {
                    cmd.args(args);

                    match level {
                        Some(level) => cmd.env("PROTO_LOG", level),
                        None => cmd.env_remove("PROTO_LOG"),
                    };
                })
                .stderr()
        }

        #[test]
        fn tool_commands_only_log_warnings_by_default() {
            for args in TOOL_COMMANDS {
                let stderr = get_stderr(args, None);

                assert!(!stderr.contains(INFO_LOG), "{args:?}: {stderr}");
            }
        }

        #[test]
        fn tool_commands_use_an_explicit_level() {
            for args in TOOL_COMMANDS {
                let stderr = get_stderr(args, Some("info"));

                assert!(stderr.contains(INFO_LOG), "{args:?}: {stderr}");
            }
        }

        #[test]
        fn log_arg_takes_precedence_over_env_var() {
            let stderr = get_stderr(&["run", "protostar", "--log", "info"], Some("warn"));

            assert!(stderr.contains(INFO_LOG), "{stderr}");
        }

        #[test]
        fn other_commands_log_info_by_default() {
            let stderr = get_stderr(&["versions", "protostar"], None);

            assert!(stderr.contains(INFO_LOG), "{stderr}");
        }

        #[test]
        fn other_commands_use_an_explicit_level() {
            let stderr = get_stderr(&["versions", "protostar"], Some("warn"));

            assert!(!stderr.contains(INFO_LOG), "{stderr}");
        }
    }
}
