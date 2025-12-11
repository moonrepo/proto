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

    #[test]
    fn cleans_multiple_stale_tool_versions() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("2.0.0");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("3.0.0");
            })
            .success();

        // Calculate timestamps - stale versions should have last-used time > 2 days ago
        let now_millis = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let stale_time = now_millis - (86400 * 2 * 1000); // 2 days ago in milliseconds

        // Set stale last-used timestamps for versions 1.0.0 and 2.0.0
        sandbox.create_file(
            ".proto/tools/protostar/1.0.0/.last-used",
            &stale_time.to_string(),
        );
        sandbox.create_file(
            ".proto/tools/protostar/2.0.0/.last-used",
            &stale_time.to_string(),
        );
        // Version 3.0.0 is recent (within 1 day)
        sandbox.create_file(
            ".proto/tools/protostar/3.0.0/.last-used",
            &now_millis.to_string(),
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("clean")
                    .arg("--yes")
                    .arg("tools")
                    .arg("--days")
                    .arg("1");
            })
            .success();

        assert!(!sandbox.path().join(".proto/tools/protostar/1.0.0").exists());
        assert!(!sandbox.path().join(".proto/tools/protostar/2.0.0").exists());
        assert!(sandbox.path().join(".proto/tools/protostar/3.0.0").exists());
    }
}
