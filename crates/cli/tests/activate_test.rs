use proto_core::test_utils::*;
use starbase_sandbox::{Sandbox, SandboxAssert, assert_snapshot};
use starbase_shell::ShellType;

fn get_activate_output(assert: &SandboxAssert, sandbox: &Sandbox) -> String {
    let root = sandbox.path().to_str().unwrap();

    assert.output().replace(root, "/sandbox")
}

mod activate {
    use super::*;

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
