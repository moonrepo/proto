// Different snapshot output on Windows!
#[cfg(unix)]
mod activate {
    use proto_core::test_utils::*;
    use starbase_sandbox::predicates::prelude::*;
    use starbase_sandbox::{Sandbox, SandboxAssert, assert_snapshot};
    use starbase_shell::ShellType;

    fn get_activate_output(assert: &SandboxAssert, sandbox: &Sandbox) -> String {
        let root = sandbox.path().to_str().unwrap();

        assert.output().replace(root, "/sandbox")
    }

    #[test]
    fn empty_output_if_no_tools() {
        let sandbox = create_empty_proto_sandbox();

        for shell in ShellType::variants() {
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate").arg(shell.to_string());
            });

            assert_snapshot!(get_activate_output(&assert, &sandbox));
        }
    }

    #[test]
    fn passes_args_through() {
        let sandbox = create_empty_proto_sandbox();

        for shell in ShellType::variants() {
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg(shell.to_string())
                    .arg("--config-mode")
                    .arg("upwards-global")
                    .arg("--no-shim")
                    .arg("--no-bin");
            });

            assert_snapshot!(get_activate_output(&assert, &sandbox));
        }
    }

    #[test]
    fn supports_json_exports() {
        let sandbox = create_empty_proto_sandbox();

        // Only nushell supports JSON!
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("activate")
                .arg("nu")
                .arg("--config-mode")
                .arg("upwards-global");
        });

        assert_snapshot!(get_activate_output(&assert, &sandbox));
    }

    #[test]
    fn supports_one_tool() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".prototools", r#"protostar = "1.0.0""#);

        for shell in ShellType::variants() {
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate").arg(shell.to_string());
            });

            assert_snapshot!(get_activate_output(&assert, &sandbox));
        }
    }

    #[test]
    fn supports_many_tools() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
protostar = "1.0.0"
moonstone = "2.0.0"
"#,
        );

        for shell in ShellType::variants() {
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate").arg(shell.to_string());
            });

            assert_snapshot!(get_activate_output(&assert, &sandbox));
        }
    }

    #[test]
    fn can_include_global_tools() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/.prototools", r#"protostar = "1.0.0""#);
        sandbox.create_file(".prototools", r#"moonstone = "2.0.0""#);

        for shell in ShellType::variants() {
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg(shell.to_string())
                    .arg("--export")
                    .arg("--config-mode")
                    .arg("all"); // upwards-global
            });

            assert_snapshot!(get_activate_output(&assert, &sandbox));
        }
    }

    #[test]
    fn can_disable_init() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".prototools", r#"protostar = "1.0.0""#);

        for shell in ShellType::variants() {
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate").arg(shell.to_string()).arg("--no-init");
            });

            assert_snapshot!(get_activate_output(&assert, &sandbox));
        }
    }

    mod ai_agent {
        use super::*;

        #[test]
        fn prints_hook_by_default() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg("zsh")
                    .env("CODEX_CI", "1")
                    .env_remove("PROTO_REPORTER");
            });
            assert.success().stdout(
                predicate::str::contains("_proto_activate_hook")
                    .and(predicate::str::contains("{\"type\":").not())
                    .and(predicate::str::contains("Detected an AI agent").not()),
            );
        }

        #[test]
        fn prints_shell_syntax_for_export() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg("zsh")
                    .arg("--export")
                    .env("CODEX_CI", "1")
                    .env_remove("PROTO_REPORTER");
            });
            assert.success().stdout(
                predicate::str::contains("_PROTO_ACTIVATED_PATH")
                    .and(predicate::str::contains("{\"type\":").not()),
            );
        }

        #[test]
        fn prints_plain_json_for_explicit_json() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg("zsh")
                    .arg("--json")
                    .env("CODEX_CI", "1")
                    .env_remove("PROTO_REPORTER");
            });
            let stdout = assert.stdout();
            assert.success();
            let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();

            assert!(output.get("env").is_some());
            assert!(output.get("path").is_some());
        }

        #[test]
        fn prints_agent_notice_and_data_for_explicit_reporter() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg("zsh")
                    .arg("--reporter")
                    .arg("ndjson")
                    .env("CODEX_CI", "1");
            });
            let stdout = assert.stdout();
            let records = stdout
                .lines()
                .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
                .collect::<Vec<_>>();

            assert.success();
            assert!(records.iter().any(|record| {
                record.get("type").and_then(|value| value.as_str()) == Some("message")
                    && record
                        .get("message")
                        .and_then(|value| value.as_str())
                        .is_some_and(|message| message.contains("Detected an AI agent"))
            }));
            assert!(records.iter().any(|record| {
                record.get("type").and_then(|value| value.as_str()) == Some("data")
            }));
        }

        #[test]
        fn nu_hook_requests_explicit_json() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg("nu")
                    .env("CODEX_CI", "1")
                    .env_remove("PROTO_REPORTER");
            });
            assert.success().stdout(
                predicate::str::contains("proto activate nu --reporter json")
                    .and(predicate::str::contains("\"type\":").not()),
            );

            // The nested call the hook makes must parse as plain JSON
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg("nu")
                    .arg("--reporter")
                    .arg("json")
                    .env("CODEX_CI", "1");
            });
            let stdout = assert.stdout();
            assert.success();
            let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();

            assert!(output.get("env").is_some());
        }

        #[test]
        fn keeps_stdout_empty_when_export_fails() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg("zsh")
                    .arg("--export")
                    .arg("--reporter")
                    .arg("ndjson")
                    .env("CODEX_CI", "1")
                    // An unusable store location makes the command fail
                    .env("PROTO_HOME", "/dev/null/proto")
                    .env_remove("PROTO_SANDBOX")
                    .env_remove("PROTO_TEST");
            });
            let stderr = assert.stderr();
            let records = stderr
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| {
                    serde_json::from_str::<serde_json::Value>(line).unwrap_or_else(|error| {
                        panic!("stderr line is not valid NDJSON: {line}: {error}")
                    })
                })
                .collect::<Vec<_>>();

            // The failure must render on stderr because stdout is evaluated.
            assert.failure().stdout(predicate::str::is_empty());
            assert!(records.iter().any(|record| {
                record.get("type").and_then(|value| value.as_str()) == Some("error")
            }));
        }

        #[test]
        fn export_wins_over_structured_reporter() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg("zsh")
                    .arg("--export")
                    .arg("--reporter")
                    .arg("ndjson")
                    .env("CODEX_CI", "1");
            });
            assert.success().stdout(
                predicate::str::contains("_PROTO_ACTIVATED_PATH")
                    .and(predicate::str::contains("{\"type\":").not()),
            );
        }

        #[test]
        fn keeps_ndjson_tracing_for_shell_output() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg("zsh")
                    .arg("--export")
                    .arg("--reporter")
                    .arg("ndjson")
                    .arg("--log")
                    .arg("debug");
            });
            let stderr = assert.stderr();

            assert.success();
            assert!(!stderr.trim().is_empty());

            for line in stderr.lines().filter(|line| !line.trim().is_empty()) {
                serde_json::from_str::<serde_json::Value>(line).unwrap_or_else(|error| {
                    panic!("stderr line is not valid NDJSON: {line}: {error}")
                });
            }
        }
    }

    mod export {
        use super::*;

        #[test]
        fn includes_shared_env_if_no_tools() {
            let sandbox = create_empty_proto_sandbox();

            sandbox.create_file(
                ".prototools",
                r#"
[env]
KEY = "value"
"#,
            );

            for shell in ShellType::variants() {
                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("activate").arg(shell.to_string()).arg("--export");
                });

                assert_snapshot!(get_activate_output(&assert, &sandbox));
            }
        }

        #[test]
        fn includes_shell_aliases_if_no_tools() {
            let sandbox = create_empty_proto_sandbox();

            sandbox.create_file(
                ".prototools",
                r#"
[shell.aliases]
gs = "git status"
".." = "cd .."
"#,
            );

            for shell in ShellType::variants() {
                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("activate").arg(shell.to_string()).arg("--export");
                });

                assert_snapshot!(get_activate_output(&assert, &sandbox));
            }
        }

        #[test]
        fn includes_tool_env() {
            let sandbox = create_empty_proto_sandbox();

            sandbox.create_file(
                ".prototools",
                r#"
protostar = "1.0.0"

[env]
KEY1 = "value1"

[tools.protostar.env]
KEY2 = "value2"
"#,
            );

            for shell in ShellType::variants() {
                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("activate").arg(shell.to_string()).arg("--export");
                });

                assert_snapshot!(get_activate_output(&assert, &sandbox));
            }
        }

        #[test]
        fn can_include_global_tools() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".proto/.prototools", r#"protostar = "1.0.0""#);
            sandbox.create_file(".prototools", r#"moonstone = "2.0.0""#);

            for shell in ShellType::variants() {
                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("activate")
                        .arg(shell.to_string())
                        .arg("--export")
                        .arg("--config-mode")
                        .arg("all"); // upwards-global
                });

                assert_snapshot!(get_activate_output(&assert, &sandbox));
            }
        }

        #[test]
        fn tracks_used_at() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", r#"protostar = "1.0.0""#);

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("1.0.0");
                })
                .success();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("activate").arg("zsh").arg("--export");
                })
                .success();

            assert!(
                sandbox
                    .path()
                    .join(".proto/tools/protostar/1.0.0/.last-used")
                    .exists()
            );
        }
    }
}
