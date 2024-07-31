mod utils;

// Different snapshot output on Windows!
#[cfg(unix)]
mod activate {
    use crate::utils::*;
    use starbase_sandbox::{assert_snapshot, Sandbox, SandboxAssert};

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
                .arg("--include-global")
                .arg("--no-shim")
                .arg("--no-bin");
        });

        assert_snapshot!(get_activate_output(&assert, &sandbox));
    }

    #[test]
    fn supports_json_exports() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("activate").arg("nu").arg("--include-global");
        });

        assert_snapshot!(get_activate_output(&assert, &sandbox));
    }

    #[test]
    fn supports_one_tool() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".prototools", r#"node = "20.0.0""#);

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
node = "20.0.0"
yarn = "4.0.0"
bun = "1.1.0"
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
        sandbox.create_file(".proto/.prototools", r#"npm = "10.0.0""#);
        sandbox.create_file(".prototools", r#"pnpm = "8.0.0""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("activate")
                .arg("elvish")
                .arg("--export")
                .arg("--include-global");
        });

        assert_snapshot!(get_activate_output(&assert, &sandbox));
    }
}
