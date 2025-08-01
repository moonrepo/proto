mod utils;

// Different snapshot output on Windows!
#[cfg(unix)]
mod activate {
    use crate::utils::*;
    use starbase_sandbox::{Sandbox, SandboxAssert, assert_snapshot};

    fn get_activate_output(assert: &SandboxAssert, sandbox: &Sandbox) -> String {
        let root = sandbox.path().to_str().unwrap();

        assert.output().replace(root, "/sandbox")
    }

    #[test]
    fn empty_output_if_no_tools() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("activate").arg("bash");
        });

        assert_snapshot!(get_activate_output(&assert, &sandbox));
    }

    #[test]
    fn passes_args_through() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("activate")
                .arg("elvish")
                .arg("--config-mode")
                .arg("upwards-global")
                .arg("--no-shim")
                .arg("--no-bin");
        });

        assert_snapshot!(get_activate_output(&assert, &sandbox));
    }

    #[test]
    fn supports_json_exports() {
        let sandbox = create_empty_proto_sandbox();

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

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("activate").arg("zsh");
        });

        assert_snapshot!(get_activate_output(&assert, &sandbox));
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

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("activate").arg("fish");
        });

        assert_snapshot!(get_activate_output(&assert, &sandbox));
    }

    #[test]
    fn can_include_global_tools() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/.prototools", r#"protostar = "1.0.0""#);
        sandbox.create_file(".prototools", r#"moonstone = "2.0.0""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("activate")
                .arg("elvish")
                .arg("--export")
                .arg("--config-mode")
                .arg("all"); // upwards-global
        });

        let output = get_activate_output(&assert, &sandbox);

        assert!(output.contains("<WORKSPACE>/.proto/activate-start <WORKSPACE>/.proto/shims <WORKSPACE>/.proto/bin <WORKSPACE>/.proto/activate-stop"));
    }

    #[test]
    fn can_disable_init() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".prototools", r#"protostar = "1.0.0""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("activate").arg("zsh").arg("--no-init");
        });

        assert_snapshot!(get_activate_output(&assert, &sandbox));
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

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate").arg("bash").arg("--export");
            });

            let output = get_activate_output(&assert, &sandbox);

            assert!(output.contains("export KEY=value;"));
            assert!(output.contains("export _PROTO_ACTIVATED_ENV=KEY;"));
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

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate").arg("bash").arg("--export");
            });

            let output = get_activate_output(&assert, &sandbox);

            assert!(output.contains("export KEY1=value1;"));
            assert!(output.contains("export KEY2=value2;"));
            assert!(output.contains("export _PROTO_ACTIVATED_ENV=KEY1,KEY2;"));
        }

        #[test]
        fn can_include_global_tools() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".proto/.prototools", r#"protostar = "1.0.0""#);
            sandbox.create_file(".prototools", r#"moonstone = "2.0.0""#);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("activate")
                    .arg("elvish")
                    .arg("--export")
                    .arg("--config-mode")
                    .arg("all"); // upwards-global
            });

            let output = get_activate_output(&assert, &sandbox);

            assert!(output.contains("<WORKSPACE>/.proto/activate-start <WORKSPACE>/.proto/shims <WORKSPACE>/.proto/bin <WORKSPACE>/.proto/activate-stop"));
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
