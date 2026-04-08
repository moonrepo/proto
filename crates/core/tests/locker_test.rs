use proto_core::{
    Id, LockRecord, ProtoConfig, ProtoEnvironment, ProtoLock, Tool, ToolContext, ToolSpec,
    flow::lock::Locker, load_tool_from_locator,
};
use proto_pdk_api::Checksum;
use starbase_sandbox::create_empty_sandbox;
use std::path::Path;
use system_env::{SystemArch, SystemOS};
use version_spec::{UnresolvedVersionSpec, VersionSpec};

async fn create_tool_in_sandbox(sandbox_path: &Path) -> Tool {
    let mut proto = ProtoEnvironment::new_testing(sandbox_path).unwrap();
    proto.working_dir = sandbox_path.to_path_buf();

    load_tool_from_locator(
        ToolContext::parse("node").unwrap(),
        proto,
        ProtoConfig::default()
            .builtin_plugins()
            .tools
            .get("node")
            .unwrap(),
    )
    .await
    .unwrap()
}

fn make_record(
    version: &str,
    spec: &str,
    os: Option<SystemOS>,
    arch: Option<SystemArch>,
) -> LockRecord {
    LockRecord {
        version: Some(VersionSpec::parse(version).unwrap()),
        spec: Some(UnresolvedVersionSpec::parse(spec).unwrap()),
        os,
        arch,
        ..Default::default()
    }
}

mod locker {
    use super::*;

    mod resolve_locked_record {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_none_when_no_lockfile() {
            let sandbox = create_empty_sandbox();
            // No lockfile setting, so load_lock returns None
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let spec = ToolSpec::parse("20.0.0").unwrap();
            let result = locker.resolve_locked_record(&spec).unwrap();

            assert!(result.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_matching_record() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            // Pre-create a lockfile with a record
            let os = SystemOS::default();
            let arch = SystemArch::default();
            let mut lock = ProtoLock::default();
            lock.tools
                .entry(Id::raw("node"))
                .or_default()
                .push(LockRecord {
                    version: Some(VersionSpec::parse("20.0.0").unwrap()),
                    spec: Some(UnresolvedVersionSpec::parse("20.0.0").unwrap()),
                    os: Some(os),
                    arch: Some(arch),
                    ..Default::default()
                });
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let spec = ToolSpec::parse("20.0.0").unwrap();
            let result = locker.resolve_locked_record(&spec).unwrap();

            assert!(result.is_some());
            let record = result.unwrap();
            assert_eq!(record.version, Some(VersionSpec::parse("20.0.0").unwrap()));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_none_when_no_matching_record() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            // Create lockfile with a different version
            let mut lock = ProtoLock::default();
            lock.tools
                .entry(Id::raw("node"))
                .or_default()
                .push(LockRecord {
                    version: Some(VersionSpec::parse("18.0.0").unwrap()),
                    spec: Some(UnresolvedVersionSpec::parse("18.0.0").unwrap()),
                    os: Some(SystemOS::default()),
                    arch: Some(SystemArch::default()),
                    ..Default::default()
                });
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let spec = ToolSpec::parse("20.0.0").unwrap();
            let result = locker.resolve_locked_record(&spec).unwrap();

            assert!(result.is_none());
        }
    }

    mod insert_record {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inserts_new_record() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let record = make_record(
                "20.0.0",
                "20.0.0",
                Some(SystemOS::default()),
                Some(SystemArch::default()),
            );

            locker.insert_record_into_lockfile(&record).unwrap();

            // Verify it was persisted
            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].version,
                Some(VersionSpec::parse("20.0.0").unwrap())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn replaces_record_with_higher_version() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            // Pre-create lockfile with v20.0.0
            let os = SystemOS::default();
            let arch = SystemArch::default();
            let mut lock = ProtoLock::default();
            lock.tools
                .entry(Id::raw("node"))
                .or_default()
                .push(LockRecord {
                    version: Some(VersionSpec::parse("20.0.0").unwrap()),
                    spec: Some(UnresolvedVersionSpec::parse("20.0.0").unwrap()),
                    os: Some(os),
                    arch: Some(arch),
                    ..Default::default()
                });
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            // Insert higher version with same spec
            let record = make_record("20.1.0", "20.0.0", Some(os), Some(arch));
            locker.insert_record_into_lockfile(&record).unwrap();

            // Should have replaced (still 1 record)
            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].version,
                Some(VersionSpec::parse("20.1.0").unwrap())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_op_when_no_lockfile_config() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let record = make_record(
                "20.0.0",
                "20.0.0",
                Some(SystemOS::default()),
                Some(SystemArch::default()),
            );

            // Should not error, just no-op
            locker.insert_record_into_lockfile(&record).unwrap();

            // No lockfile should exist
            assert!(!sandbox.path().join(".protolock").exists());
        }
    }

    mod remove_version {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_matching_version() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            // Create lockfile with two records
            let os = SystemOS::default();
            let arch = SystemArch::default();
            let mut lock = ProtoLock::default();
            let records = lock.tools.entry(Id::raw("node")).or_default();
            records.push(LockRecord {
                version: Some(VersionSpec::parse("18.0.0").unwrap()),
                spec: Some(UnresolvedVersionSpec::parse("18.0.0").unwrap()),
                os: Some(os),
                arch: Some(arch),
                ..Default::default()
            });
            records.push(LockRecord {
                version: Some(VersionSpec::parse("20.0.0").unwrap()),
                spec: Some(UnresolvedVersionSpec::parse("20.0.0").unwrap()),
                os: Some(os),
                arch: Some(arch),
                ..Default::default()
            });
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            locker
                .remove_version_from_lockfile(&VersionSpec::parse("18.0.0").unwrap())
                .unwrap();

            // Only v20 should remain
            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].version,
                Some(VersionSpec::parse("20.0.0").unwrap())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_tool_entry_when_last_version() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            let os = SystemOS::default();
            let arch = SystemArch::default();
            let mut lock = ProtoLock::default();
            lock.tools
                .entry(Id::raw("node"))
                .or_default()
                .push(LockRecord {
                    version: Some(VersionSpec::parse("20.0.0").unwrap()),
                    spec: Some(UnresolvedVersionSpec::parse("20.0.0").unwrap()),
                    os: Some(os),
                    arch: Some(arch),
                    ..Default::default()
                });
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            locker
                .remove_version_from_lockfile(&VersionSpec::parse("20.0.0").unwrap())
                .unwrap();

            // Tool entry should be gone, and lockfile deleted since empty
            assert!(!sandbox.path().join(".protolock").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_op_when_version_not_found() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            let os = SystemOS::default();
            let arch = SystemArch::default();
            let mut lock = ProtoLock::default();
            lock.tools
                .entry(Id::raw("node"))
                .or_default()
                .push(LockRecord {
                    version: Some(VersionSpec::parse("20.0.0").unwrap()),
                    spec: Some(UnresolvedVersionSpec::parse("20.0.0").unwrap()),
                    os: Some(os),
                    arch: Some(arch),
                    ..Default::default()
                });
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            // Remove a version that doesn't exist
            locker
                .remove_version_from_lockfile(&VersionSpec::parse("16.0.0").unwrap())
                .unwrap();

            // Original record still exists
            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();
            assert_eq!(records.len(), 1);
        }
    }

    mod remove_from_lockfile {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_entire_tool() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            let os = SystemOS::default();
            let arch = SystemArch::default();
            let mut lock = ProtoLock::default();
            lock.tools
                .entry(Id::raw("node"))
                .or_default()
                .push(LockRecord {
                    version: Some(VersionSpec::parse("20.0.0").unwrap()),
                    spec: Some(UnresolvedVersionSpec::parse("20.0.0").unwrap()),
                    os: Some(os),
                    arch: Some(arch),
                    ..Default::default()
                });
            // Add another tool to keep the file around
            lock.tools
                .entry(Id::raw("bun"))
                .or_default()
                .push(LockRecord {
                    version: Some(VersionSpec::parse("1.0.0").unwrap()),
                    spec: Some(UnresolvedVersionSpec::parse("1.0.0").unwrap()),
                    ..Default::default()
                });
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            locker.remove_from_lockfile().unwrap();

            // Node should be gone, bun should remain
            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            assert!(!lock.tools.contains_key(&Id::raw("node")));
            assert!(lock.tools.contains_key(&Id::raw("bun")));
        }
    }

    mod verify_locked_record {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn passes_when_no_locked_record() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let spec = ToolSpec::parse("20.0.0").unwrap();
            let install_record = make_record("20.0.0", "20.0.0", None, None);

            // No locked record means verification passes
            locker.verify_locked_record(&spec, &install_record).unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn passes_when_checksums_match() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let checksum = Checksum::sha256("abc123".into());

            // Set up a spec with version_locked containing a checksum
            let mut spec = ToolSpec::parse("20.0.0").unwrap();
            spec.version_locked = Some(LockRecord {
                checksum: Some(checksum.clone()),
                ..Default::default()
            });

            let install_record = LockRecord {
                checksum: Some(checksum),
                ..Default::default()
            };

            locker.verify_locked_record(&spec, &install_record).unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fails_when_checksums_mismatch() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let mut spec = ToolSpec::parse("20.0.0").unwrap();
            spec.version_locked = Some(LockRecord {
                checksum: Some(Checksum::sha256("expected_hash".into())),
                ..Default::default()
            });

            let install_record = LockRecord {
                checksum: Some(Checksum::sha256("actual_hash".into())),
                ..Default::default()
            };

            let result = locker.verify_locked_record(&spec, &install_record);
            assert!(result.is_err());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_verification_when_different_backends() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let mut spec = ToolSpec::parse("20.0.0").unwrap();
            spec.version_locked = Some(LockRecord {
                backend: Some(Id::raw("proto")),
                checksum: Some(Checksum::sha256("expected_hash".into())),
                ..Default::default()
            });

            let install_record = LockRecord {
                backend: Some(Id::raw("asdf")),
                checksum: Some(Checksum::sha256("different_hash".into())),
                ..Default::default()
            };

            // Different backends, should skip verification
            locker.verify_locked_record(&spec, &install_record).unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fails_on_os_mismatch() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let mut spec = ToolSpec::parse("20.0.0").unwrap();
            spec.version_locked = Some(LockRecord {
                os: Some(SystemOS::Linux),
                ..Default::default()
            });

            let install_record = LockRecord {
                os: Some(SystemOS::MacOS),
                ..Default::default()
            };

            let result = locker.verify_locked_record(&spec, &install_record);
            assert!(result.is_err());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fails_on_arch_mismatch() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let mut spec = ToolSpec::parse("20.0.0").unwrap();
            spec.version_locked = Some(LockRecord {
                arch: Some(SystemArch::X64),
                ..Default::default()
            });

            let install_record = LockRecord {
                arch: Some(SystemArch::Arm64),
                ..Default::default()
            };

            let result = locker.verify_locked_record(&spec, &install_record);
            assert!(result.is_err());
        }
    }

    mod get_resolved_locked_record {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_version_locked_from_spec() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let record = LockRecord {
                version: Some(VersionSpec::parse("20.0.0").unwrap()),
                checksum: Some(Checksum::sha256("test".into())),
                ..Default::default()
            };

            let mut spec = ToolSpec::parse("20.0.0").unwrap();
            spec.version_locked = Some(record.clone());

            let result = locker.get_resolved_locked_record(&spec);
            assert!(result.is_some());
            assert_eq!(
                result.unwrap().checksum,
                Some(Checksum::sha256("test".into()))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_none_when_no_locked_data() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let spec = ToolSpec::parse("20.0.0").unwrap();
            let result = locker.get_resolved_locked_record(&spec);

            assert!(result.is_none());
        }
    }
}
