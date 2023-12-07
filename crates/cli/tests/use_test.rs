mod utils;

use utils::*;

mod install_all {
    use super::*;

    #[test]
    fn installs_all_tools() {
        let sandbox = create_empty_sandbox();
        let node_path = sandbox.path().join(".proto/tools/node/19.0.0");
        let npm_path = sandbox.path().join(".proto/tools/npm/9.0.0");
        let deno_path = sandbox.path().join(".proto/tools/deno/1.30.0");

        sandbox.create_file(
            ".prototools",
            r#"node = "19.0.0"
npm = "9.0.0"
deno = "1.30.0"
    "#,
        );

        assert!(!node_path.exists());
        assert!(!npm_path.exists());
        assert!(!deno_path.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("use").assert().success();

        assert!(node_path.exists());
        assert!(npm_path.exists());
        assert!(deno_path.exists());
    }

    #[test]
    fn installs_tool_via_detection() {
        let sandbox = create_empty_sandbox();
        let node_path = sandbox.path().join(".proto/tools/node/19.0.0");

        sandbox.create_file(".nvmrc", "19.0.0");

        assert!(!node_path.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("use").assert().success();

        assert!(node_path.exists());
    }

    #[test]
    fn doesnt_install_global_tools() {
        let sandbox = create_empty_sandbox();
        let node_path = sandbox.path().join(".proto/tools/node/19.0.0");
        let deno_path = sandbox.path().join(".proto/tools/deno/1.30.0");

        sandbox.create_file(".prototools", r#"node = "19.0.0""#);
        sandbox.create_file(".proto/.prototools", r#"deno = "1.30.0""#);

        assert!(!node_path.exists());
        assert!(!deno_path.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("use").assert().success();

        assert!(node_path.exists());
        assert!(!deno_path.exists());
    }
}
