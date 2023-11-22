mod utils;

use utils::*;

mod clean {
    use super::*;

    #[test]
    fn cleans_without_issue() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("clean").arg("--yes").assert().success();
    }

    #[test]
    fn purges_tool_inventory() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("tools/node/1.2.3/index.js", "");
        sandbox.create_file("tools/node/4.5.6/index.js", "");

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("clean")
            .arg("--yes")
            .arg("--purge")
            .arg("node")
            .assert()
            .success();

        assert!(!sandbox.path().join("tools/node/1.2.3/index.js").exists());
        assert!(!sandbox.path().join("tools/node/4.5.6/index.js").exists());
    }

    #[cfg(not(windows))]
    #[test]
    fn purges_tool_bin() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("tools/node/fake/file", "");
        sandbox.create_file("bin/other", "");

        let bin = sandbox.path().join(if cfg!(windows) {
            "bin/node.exe"
        } else {
            "bin/node"
        });

        #[allow(deprecated)]
        std::fs::soft_link(sandbox.path().join("tools/node/fake/file"), &bin).unwrap();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("clean")
            .arg("--yes")
            .arg("--purge")
            .arg("node")
            .assert()
            .success();

        assert!(!bin.exists());
        assert!(bin.symlink_metadata().is_err());
    }

    #[test]
    fn purges_tool_shims() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("shims/npm", "");
        sandbox.create_file("shims/npm.cmd", "");
        sandbox.create_file("shims/npx", "");
        sandbox.create_file("shims/npx.cmd", "");

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("clean")
            .arg("--yes")
            .arg("--purge")
            .arg("npm")
            .assert()
            .success();

        if cfg!(windows) {
            assert!(!sandbox.path().join("shims/npm.cmd").exists());
            assert!(!sandbox.path().join("shims/npx.cmd").exists());
        } else {
            assert!(!sandbox.path().join("shims/npm").exists());
            assert!(!sandbox.path().join("shims/npx").exists());
        }
    }

    #[test]
    fn purges_plugins() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("plugins/node_plugin.wasm", "");
        sandbox.create_file("plugins/npm_plugin.wasm", "");

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("clean")
            .arg("--yes")
            .arg("--purge-plugins")
            .assert()
            .success();

        assert!(!sandbox.path().join("plugins/node_plugin.wasm").exists());
        assert!(!sandbox.path().join("plugins/npm_plugin.wasm").exists());
    }
}
