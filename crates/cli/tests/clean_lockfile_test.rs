use proto_core::test_utils::*;
use proto_core::{Id, ProtoLock, UnresolvedVersionSpec, VersionSpec};
use std::time::{Duration, SystemTime};

mod clean_lockfile {
    use super::*;

    #[test]
    fn keeps_lockfile_records_when_cleaning_stale_versions() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"protostar = "1.0.0"

[settings]
lockfile = true
builtin-backends = false
builtin-tools = false
"#,
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("protostar")
                    .arg("1.0.0")
                    .timeout(Duration::from_mins(5));
            })
            .success();

        // Installing creates a lock record
        let lock = ProtoLock::load_from(sandbox.path()).unwrap();
        let records = lock.tools.get(&Id::raw("protostar")).unwrap();

        assert_eq!(records.len(), 1);

        // Mark the version as stale (2 days ago)
        let now_millis = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let stale_time = now_millis - (86400 * 2 * 1000);

        sandbox.create_file(
            ".proto/tools/protostar/1.0.0/.last-used",
            stale_time.to_string(),
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("clean")
                    .arg("--yes")
                    .arg("tools")
                    .arg("--days")
                    .arg("1")
                    .timeout(Duration::from_mins(5));
            })
            .success();

        // The version was uninstalled
        assert!(!sandbox.path().join(".proto/tools/protostar/1.0.0").exists());

        // But the lock record must remain, as the config still requires it
        let lock = ProtoLock::load_from(sandbox.path()).unwrap();
        let records = lock.tools.get(&Id::raw("protostar")).unwrap();

        assert_eq!(records.len(), 1);

        let record = records.first().unwrap();

        assert_eq!(
            record.spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse("1.0.0").unwrap()
        );
        assert_eq!(
            record.version.as_ref().unwrap(),
            &VersionSpec::parse("1.0.0").unwrap()
        );
        assert!(record.checksum.is_some());
    }
}
