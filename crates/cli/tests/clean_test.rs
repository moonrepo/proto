mod utils;

use std::fs;
use std::time::{Duration, SystemTime};
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
    fn cleans_plugins() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/plugins/a_plugin.wasm", "{}");
        sandbox.create_file(".proto/plugins/b_plugin.wasm", "{}");

        fs::File::options()
            .write(true)
            .open(sandbox.path().join(".proto/plugins/a_plugin.wasm"))
            .unwrap()
            .set_times(
                fs::FileTimes::new().set_accessed(
                    SystemTime::now()
                        .checked_sub(Duration::from_secs(86400 * 2))
                        .unwrap(),
                ),
            )
            .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("clean")
                    .arg("--yes")
                    .arg("plugins")
                    .arg("--days")
                    .arg("1");
            })
            .success();

        assert!(!sandbox.path().join(".proto/plugins/a_plugin.wasm").exists());
        assert!(sandbox.path().join(".proto/plugins/b_plugin.wasm").exists());
    }
}
