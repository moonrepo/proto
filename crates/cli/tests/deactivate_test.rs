// Different snapshot output on Windows!
#[cfg(unix)]
mod deactivate {
    use proto_core::test_utils::*;
    use starbase_sandbox::assert_cmd::Command;
    use starbase_sandbox::predicates::prelude::*;
    use starbase_sandbox::{Sandbox, SandboxAssert, assert_snapshot};
    use starbase_shell::ShellType;

    fn get_deactivate_output(assert: &SandboxAssert, sandbox: &Sandbox) -> String {
        let root = sandbox.path().to_str().unwrap();

        assert.output().replace(root, "/sandbox")
    }

    // Pretend that a previous activation set these variables and aliases.
    fn activated(cmd: &mut Command, sandbox: &Sandbox) {
        cmd.env("_PROTO_ACTIVATED_ENV", "PROTO_HOME,KEY1,KEY2");
        cmd.env("_PROTO_ACTIVATED_ALIASES", "pn,yn");
        cmd.env(
            "_PROTO_ACTIVATED_PATH",
            format!("{}/.proto/shims", sandbox.path().display()),
        );
    }

    // And that it injected paths between the boundary markers.
    fn activated_path(cmd: &mut Command, sandbox: &Sandbox) {
        let root = sandbox.path().display();

        cmd.env(
            "PATH",
            format!(
                "{root}/.proto/activate-start:{root}/.proto/shims:{root}/.proto/bin:{root}/.proto/activate-stop:/usr/bin:/bin"
            ),
        );
    }

    #[test]
    fn teardown_only_if_not_activated() {
        let sandbox = create_empty_proto_sandbox();

        // The hook teardown is always printed, so that deactivating fully
        // unregisters even when nothing is currently applied
        for shell in ShellType::variants() {
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("deactivate").arg(shell.to_string());
                cmd.env("PATH", "/usr/bin:/bin");
            });

            assert_snapshot!(get_deactivate_output(&assert, &sandbox));
        }
    }

    #[test]
    fn unsets_env_and_aliases() {
        let sandbox = create_empty_proto_sandbox();

        for shell in ShellType::variants() {
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("deactivate").arg(shell.to_string()).arg("--export");
                cmd.env("PATH", "/usr/bin:/bin");
                activated(cmd, &sandbox);
            });

            assert_snapshot!(get_deactivate_output(&assert, &sandbox));
        }
    }

    #[test]
    fn resets_path_between_markers() {
        let sandbox = create_empty_proto_sandbox();

        for shell in ShellType::variants() {
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("deactivate").arg(shell.to_string()).arg("--export");
                activated(cmd, &sandbox);
                activated_path(cmd, &sandbox);
            });

            assert_snapshot!(get_deactivate_output(&assert, &sandbox));
        }
    }

    #[test]
    fn leaves_path_alone_if_no_markers() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("deactivate").arg("zsh").arg("--export");
            cmd.env("PATH", "/usr/bin:/bin");
            activated(cmd, &sandbox);
        });

        assert
            .success()
            .stdout(predicate::str::contains("export PATH=").not());
    }

    #[test]
    fn export_matches_the_default_output() {
        let sandbox = create_empty_proto_sandbox();

        let with_flag = sandbox.run_bin(|cmd| {
            cmd.arg("deactivate").arg("zsh").arg("--export");
            activated(cmd, &sandbox);
            activated_path(cmd, &sandbox);
        });

        let without_flag = sandbox.run_bin(|cmd| {
            cmd.arg("deactivate").arg("zsh");
            activated(cmd, &sandbox);
            activated_path(cmd, &sandbox);
        });

        assert_eq!(with_flag.output(), without_flag.output());
    }

    #[test]
    fn supports_json_exports() {
        let sandbox = create_empty_proto_sandbox();

        // Nu no longer consumes this, but it stays available for tooling
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("deactivate")
                .arg("nu")
                .arg("--reporter")
                .arg("json");
            activated(cmd, &sandbox);
            activated_path(cmd, &sandbox);
        });

        assert_snapshot!(get_deactivate_output(&assert, &sandbox));
    }

    mod ai_agent {
        use super::*;

        #[test]
        fn prints_shell_syntax_by_default() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("deactivate")
                    .arg("zsh")
                    .env("CODEX_CI", "1")
                    .env_remove("PROTO_REPORTER");
                activated(cmd, &sandbox);
            });
            assert.success().stdout(
                predicate::str::contains("unset _PROTO_ACTIVATED_ENV")
                    .and(predicate::str::contains("{\"type\":").not()),
            );
        }

        #[test]
        fn nu_hook_stages_shell_syntax() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg("nu")
                    .env("CODEX_CI", "1")
                    .env_remove("PROTO_REPORTER");
            });
            assert
                .success()
                .stdout(predicate::str::contains("proto deactivate nu --export"));

            // The nested call the hook makes is staged to a file and sourced,
            // so it must be nu syntax and not a reporter payload
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("deactivate")
                    .arg("nu")
                    .arg("--export")
                    .env("CODEX_CI", "1");
                activated(cmd, &sandbox);
            });
            assert.success().stdout(
                predicate::str::contains("hide-env --ignore-errors _PROTO_ACTIVATED_ENV")
                    .and(predicate::str::contains("\"type\":").not()),
            );
        }
    }

    mod hook {
        use super::*;

        #[test]
        fn does_not_inherit_activate_only_args() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg("bash")
                    .arg("--config-mode")
                    .arg("upwards-global")
                    .arg("--no-shim")
                    .arg("--no-bin")
                    .arg("--no-init");
            });

            assert.success().stdout(
                predicate::str::contains(
                    "proto activate bash --config-mode upwards-global --no-bin --no-shim --export",
                )
                .and(predicate::str::contains("proto deactivate bash --export")),
            );
        }
    }
}
