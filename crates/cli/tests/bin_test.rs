use proto_core::test_utils::*;
use starbase_sandbox::predicates::prelude::*;

mod bin {
    use super::*;

    #[test]
    fn errors_if_not_installed() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("protostar").arg("1.0.0");
        });

        assert.failure().stderr(predicate::str::contains(
            "Unable to find an executable for protostar",
        ));
    }

    #[test]
    fn returns_path_in_text_and_agent_environments() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("protostar").arg("1.0.0");
        });

        if cfg!(windows) {
            assert.success().stdout(predicate::str::contains(
                "tools\\protostar\\1.0.0\\protostar.exe",
            ));
        } else {
            assert
                .success()
                .stdout(predicate::str::contains("tools/protostar/1.0.0/protostar"));
        }

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin")
                .arg("protostar")
                .arg("1.0.0")
                .env("CODEX_CI", "1")
                .env_remove("PROTO_REPORTER");
        });
        let stdout = assert.stdout();

        let records = stdout
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();

        assert!(records.iter().any(|record| {
            record.get("type").and_then(|value| value.as_str()) == Some("message")
                && record
                    .get("message")
                    .and_then(|value| value.as_str())
                    .is_some_and(|message| message.contains("Detected an AI agent"))
        }));
        assert!(records.iter().any(|record| {
            record.get("type").and_then(|value| value.as_str()) == Some("message")
                && record
                    .get("message")
                    .and_then(|value| value.as_str())
                    .is_some_and(|message| {
                        message.contains("protostar") && message.contains("1.0.0")
                    })
        }));

        assert.success();
    }

    #[test]
    fn returns_bin_path() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("protostar").arg("1.0.0").arg("--bin");
        });

        if cfg!(windows) {
            assert
                .success()
                .stdout(predicate::str::contains("protostar.exe"));
        } else {
            assert
                .success()
                .stdout(predicate::str::contains("protostar"));
        }
    }

    #[test]
    fn returns_shim_path() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("protostar").arg("1.0.0").arg("--shim");
        });

        if cfg!(windows) {
            assert
                .inner
                .stdout(predicate::str::contains("shims\\protostar.exe"));
        } else {
            assert
                .inner
                .stdout(predicate::str::contains("shims/protostar"));
        }
    }

    #[test]
    fn returns_exes_dir() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin")
                .arg("protostar")
                .arg("1.0.0")
                .arg("--dir")
                .arg("exes");
        });

        if cfg!(windows) {
            assert
                .inner
                .stdout(predicate::str::contains("tools\\protostar\\1.0.0\\lib"));
        } else {
            assert
                .inner
                .stdout(predicate::str::contains("tools/protostar/1.0.0/lib"));
        }
    }

    #[test]
    fn returns_globals_dir() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin")
                .arg("protostar")
                .arg("1.0.0")
                .arg("--dir")
                .arg("globals");
        });

        if cfg!(windows) {
            assert
                .inner
                .stdout(predicate::str::contains(".home\\.protostar\\bin"));
        } else {
            assert
                .inner
                .stdout(predicate::str::contains(".home/.protostar/bin"));
        }
    }
}
