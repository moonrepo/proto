mod utils;

use utils::*;

mod clean {
    use super::*;

    #[test]
    fn cleans_without_issue() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("clean").arg("--yes");
            })
            .success();
    }

    #[test]
    fn purges_tool_inventory() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/tools/node/1.2.3/index.js", "");
        sandbox.create_file(".proto/tools/node/4.5.6/index.js", "");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("clean").arg("--yes").arg("--purge").arg("node");
            })
            .success();

        assert!(!sandbox
            .path()
            .join(".proto/tools/node/1.2.3/index.js")
            .exists());
        assert!(!sandbox
            .path()
            .join(".proto/tools/node/4.5.6/index.js")
            .exists());
    }

    #[cfg(not(windows))]
    #[test]
    fn purges_tool_bin() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/tools/node/fake/file", "");
        sandbox.create_file(".proto/bin/other", "");

        let bin = sandbox.path().join(".proto").join(if cfg!(windows) {
            "bin/node.exe"
        } else {
            "bin/node"
        });

        #[allow(deprecated)]
        std::fs::soft_link(sandbox.path().join(".proto/tools/node/fake/file"), &bin).unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("clean").arg("--yes").arg("--purge").arg("node");
            })
            .success();

        assert!(!bin.exists());
        assert!(bin.symlink_metadata().is_err());
    }

    #[test]
    fn purges_tool_shims() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/shims/npm", "");
        sandbox.create_file(".proto/shims/npm.exe", "");
        sandbox.create_file(".proto/shims/npx", "");
        sandbox.create_file(".proto/shims/npx.exe", "");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("clean").arg("--yes").arg("--purge").arg("npm");
            })
            .success();

        if cfg!(windows) {
            assert!(!sandbox.path().join(".proto/shims/npm.exe").exists());
            assert!(!sandbox.path().join(".proto/shims/npx.exe").exists());
        } else {
            assert!(!sandbox.path().join(".proto/shims/npm").exists());
            assert!(!sandbox.path().join(".proto/shims/npx").exists());
        }
    }

    #[test]
    fn purges_plugins() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/plugins/node_plugin.wasm", "");
        sandbox.create_file(".proto/plugins/npm_plugin.wasm", "");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("clean").arg("--yes").arg("--purge-plugins");
            })
            .success();

        assert!(!sandbox
            .path()
            .join(".proto/plugins/node_plugin.wasm")
            .exists());
        assert!(!sandbox
            .path()
            .join(".proto/plugins/npm_plugin.wasm")
            .exists());
    }
}
