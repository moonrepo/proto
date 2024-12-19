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
