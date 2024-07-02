mod utils;

// Different snapshot output on Windows!
#[cfg(unix)]
mod activate {
    use crate::utils::*;
    use starbase_sandbox::assert_cmd::assert::Assert;
    use starbase_sandbox::{assert_snapshot, get_assert_output};

    fn get_activate_output(assert: &Assert, sandbox: &Sandbox) -> String {
        let root = sandbox.path().to_str().unwrap();

        get_assert_output(assert).replace(root, "/sandbox")
    }

    #[test]
    fn empty_output_if_no_tools() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("activate").arg("bash").assert();

        assert_snapshot!(get_activate_output(&assert, &sandbox));
    }

    #[test]
    fn supports_one_tool() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", r#"node = "20.0.0""#);

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("activate").arg("zsh").assert();

        assert_snapshot!(get_activate_output(&assert, &sandbox));
    }

    #[test]
    fn supports_many_tools() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
node = "20.0.0"
yarn = "4.0.0"
bun = "1.1.0"
"#,
        );

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("activate").arg("fish").assert();

        assert_snapshot!(get_activate_output(&assert, &sandbox));
    }
}
