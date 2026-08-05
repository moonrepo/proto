use proto_core::test_utils::*;
use starbase_sandbox::create_command_with_name;
use starbase_utils::{fs, hash};
use std::path::PathBuf;
use std::process::Output;
use std::thread;
use std::time::Duration;

// Concurrent installs of the same resolved version must block each other
// through a lock on the version-keyed temp directory, and only the first
// process to acquire it should do the actual work. These tests hold that
// lock up front so that every spawned process reaches it before any of
// them can finish. See issue #1063.
mod install_concurrent {
    use super::*;

    fn get_version_temp_dir(sandbox: &ProtoSandbox, version: &str) -> PathBuf {
        sandbox
            .path()
            .join(".proto/temp/protostar")
            .join(hash::base64::from_bytes(version))
    }

    fn spawn_install(sandbox: &ProtoSandbox, req: &str) -> thread::JoinHandle<Output> {
        let mut cmd = create_command_with_name(sandbox.path(), "proto", &sandbox.settings);
        cmd.arg("install").arg("protostar").arg(req);

        thread::spawn(move || cmd.output().unwrap())
    }

    fn join_and_count_installs(children: Vec<thread::JoinHandle<Output>>) -> usize {
        let mut installs = 0;

        for child in children {
            let output = child.join().unwrap();

            assert!(
                output.status.success(),
                "install process failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            installs += String::from_utf8_lossy(&output.stderr)
                .matches("Installing tool natively")
                .count();
        }

        installs
    }

    #[test]
    fn only_one_process_installs_when_racing_the_same_version() {
        let sandbox = create_empty_proto_sandbox();

        let mut lock = fs::lock_directory(get_version_temp_dir(&sandbox, "5.0.0")).unwrap();

        let children = (0..3)
            .map(|_| spawn_install(&sandbox, "5.0.0"))
            .collect::<Vec<_>>();

        // Give every process time to start and block on the lock
        thread::sleep(Duration::from_secs(3));

        lock.unlock().unwrap();

        assert_eq!(join_and_count_installs(children), 1);

        assert!(sandbox.path().join(".proto/tools/protostar/5.0.0").exists());
    }

    #[test]
    fn different_version_reqs_share_the_install_lock() {
        let sandbox = create_empty_proto_sandbox();

        // Both requirements resolve to version 5.10.15, so both processes
        // must derive the same lock even though the requirement strings
        // (and thus their hashes) differ
        let mut lock = fs::lock_directory(get_version_temp_dir(&sandbox, "5.10.15")).unwrap();

        let children = ["5.10.15", "~5"]
            .into_iter()
            .map(|req| spawn_install(&sandbox, req))
            .collect::<Vec<_>>();

        thread::sleep(Duration::from_secs(3));

        lock.unlock().unwrap();

        assert_eq!(join_and_count_installs(children), 1);

        assert!(
            sandbox
                .path()
                .join(".proto/tools/protostar/5.10.15")
                .exists()
        );
    }
}
