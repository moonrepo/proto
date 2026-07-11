use proto_core::test_utils::*;
use std::fs;
use std::time::{Duration, SystemTime};

mod clean {
    use super::*;
    use std::path::Path;

    fn make_stale(path: impl AsRef<Path>) {
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_times(
                fs::FileTimes::new().set_accessed(
                    SystemTime::now()
                        .checked_sub(Duration::from_secs(86400 * 2))
                        .unwrap(),
                ),
            )
            .unwrap();
    }

    fn run_clean(sandbox: &ProtoSandbox, target: &str) {
        sandbox
            .run_bin(|cmd| {
                cmd.arg("clean")
                    .arg("--yes")
                    .arg(target)
                    .arg("--days")
                    .arg("1");
            })
            .success();
    }

    #[test]
    fn cleans_without_issue() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("clean")
                    .arg("--yes")
                    .timeout(Duration::from_mins(3));
            })
            .success();
    }

    #[test]
    fn cleans_plugins() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/plugins/a_plugin.wasm", "{}");
        sandbox.create_file(".proto/plugins/b_plugin.wasm", "{}");

        make_stale(sandbox.path().join(".proto/plugins/a_plugin.wasm"));

        run_clean(&sandbox, "plugins");

        assert!(!sandbox.path().join(".proto/plugins/a_plugin.wasm").exists());
        assert!(sandbox.path().join(".proto/plugins/b_plugin.wasm").exists());
    }

    #[test]
    fn cleans_cache_subdirectories() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/cache/requests-v2/nested/stale.json", "{}");

        make_stale(
            sandbox
                .path()
                .join(".proto/cache/requests-v2/nested/stale.json"),
        );

        run_clean(&sandbox, "cache");

        assert!(!sandbox.path().join(".proto/cache/requests-v2").exists());

        // But never the cache directory itself
        assert!(sandbox.path().join(".proto/cache").exists());
    }

    #[test]
    fn keeps_fresh_files_in_cache_subdirectories() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/cache/registry/fresh.json", "{}");

        run_clean(&sandbox, "cache");

        assert!(
            sandbox
                .path()
                .join(".proto/cache/registry/fresh.json")
                .exists()
        );
    }

    #[test]
    fn doesnt_clean_dot_directories_in_cache() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/cache/.internal/stale.json", "{}");

        make_stale(sandbox.path().join(".proto/cache/.internal/stale.json"));

        run_clean(&sandbox, "cache");

        assert!(
            sandbox
                .path()
                .join(".proto/cache/.internal/stale.json")
                .exists()
        );
    }

    #[cfg(unix)]
    #[test]
    fn doesnt_follow_or_delete_symlinks_in_cache() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("external/stale.json", "{}");

        make_stale(sandbox.path().join("external/stale.json"));

        fs::create_dir_all(sandbox.path().join(".proto/cache")).unwrap();

        std::os::unix::fs::symlink(
            sandbox.path().join("external"),
            sandbox.path().join(".proto/cache/linked"),
        )
        .unwrap();

        std::os::unix::fs::symlink(
            sandbox.path().join("external/stale.json"),
            sandbox.path().join(".proto/cache/linked.json"),
        )
        .unwrap();

        run_clean(&sandbox, "cache");

        assert!(sandbox.path().join("external/stale.json").exists());
        assert!(sandbox.path().join(".proto/cache/linked").is_symlink());
        assert!(sandbox.path().join(".proto/cache/linked.json").is_symlink());
    }

    #[test]
    fn doesnt_recurse_into_temp() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/temp/tool/hash/leftover.bin", "");

        make_stale(sandbox.path().join(".proto/temp/tool/hash/leftover.bin"));

        run_clean(&sandbox, "temp");

        assert!(
            sandbox
                .path()
                .join(".proto/temp/tool/hash/leftover.bin")
                .exists()
        );
    }

    #[test]
    fn cleans_multiple_stale_tool_versions() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("protostar")
                    .arg("1.0.0")
                    .timeout(Duration::from_mins(3));
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("protostar")
                    .arg("2.0.0")
                    .timeout(Duration::from_mins(3));
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("protostar")
                    .arg("3.0.0")
                    .timeout(Duration::from_mins(3));
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
            stale_time.to_string(),
        );
        sandbox.create_file(
            ".proto/tools/protostar/2.0.0/.last-used",
            stale_time.to_string(),
        );
        // Version 3.0.0 is recent (within 1 day)
        sandbox.create_file(
            ".proto/tools/protostar/3.0.0/.last-used",
            now_millis.to_string(),
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("clean")
                    .arg("--yes")
                    .arg("tools")
                    .arg("--days")
                    .arg("1")
                    .timeout(Duration::from_mins(3));
            })
            .success();

        assert!(!sandbox.path().join(".proto/tools/protostar/1.0.0").exists());
        assert!(!sandbox.path().join(".proto/tools/protostar/2.0.0").exists());
        assert!(sandbox.path().join(".proto/tools/protostar/3.0.0").exists());
    }
}
