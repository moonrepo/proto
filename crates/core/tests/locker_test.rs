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
    create_tool_in_sandbox_at(sandbox_path, sandbox_path).await
}

async fn create_tool_in_sandbox_at(sandbox_path: &Path, working_dir: &Path) -> Tool {
    create_tool_in_sandbox_with_env(sandbox_path, working_dir, None).await
}

async fn create_tool_in_sandbox_with_env(
    sandbox_path: &Path,
    working_dir: &Path,
    env_mode: Option<&str>,
) -> Tool {
    let mut proto = ProtoEnvironment::new_testing(sandbox_path).unwrap();
    proto.working_dir = working_dir.to_path_buf();
    proto.env_mode = env_mode.map(|env| env.to_owned());

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

    mod update_spec {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn updates_matching_records_across_os_arch() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            // Pre-create a lockfile with records across multiple platforms
            let mut linux_record = make_record(
                "20.0.0",
                "^20",
                Some(SystemOS::Linux),
                Some(SystemArch::X64),
            );
            linux_record.checksum = Some(Checksum::sha256("linux_hash".into()));

            let mut macos_record = make_record(
                "20.0.0",
                "^20",
                Some(SystemOS::MacOS),
                Some(SystemArch::Arm64),
            );
            macos_record.checksum = Some(Checksum::sha256("macos_hash".into()));

            let other_record = make_record(
                "18.0.0",
                "18.0.0",
                Some(SystemOS::Linux),
                Some(SystemArch::X64),
            );

            let mut lock = ProtoLock::default();
            lock.tools.insert(
                Id::raw("node"),
                vec![linux_record, macos_record, other_record],
            );
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            locker
                .update_spec_in_lockfile(
                    &UnresolvedVersionSpec::parse("^20").unwrap(),
                    &UnresolvedVersionSpec::parse("21.1.0").unwrap(),
                    &VersionSpec::parse("21.1.0").unwrap(),
                )
                .unwrap();

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();

            assert_eq!(records.len(), 3);

            let migrated = records
                .iter()
                .filter(|record| {
                    record.spec == Some(UnresolvedVersionSpec::parse("21.1.0").unwrap())
                })
                .collect::<Vec<_>>();

            assert_eq!(migrated.len(), 2);

            for record in migrated {
                assert_eq!(record.version, Some(VersionSpec::parse("21.1.0").unwrap()));
                assert_eq!(record.checksum, None);
            }

            // Platforms are preserved
            assert!(
                records
                    .iter()
                    .any(|record| record.os == Some(SystemOS::Linux)
                        && record.arch == Some(SystemArch::X64)
                        && record.spec == Some(UnresolvedVersionSpec::parse("21.1.0").unwrap()))
            );
            assert!(
                records
                    .iter()
                    .any(|record| record.os == Some(SystemOS::MacOS)
                        && record.arch == Some(SystemArch::Arm64))
            );

            // Other specs are left untouched
            let other = records
                .iter()
                .find(|record| record.spec == Some(UnresolvedVersionSpec::parse("18.0.0").unwrap()))
                .unwrap();

            assert_eq!(other.version, Some(VersionSpec::parse("18.0.0").unwrap()));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn keeps_existing_record_matching_new_spec() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            let os = SystemOS::default();
            let arch = SystemArch::default();

            let old_record = make_record("20.0.0", "^20", Some(os), Some(arch));

            // An ad-hoc install already exists for the new spec, with a checksum
            let mut new_record = make_record("21.1.0", "21.1.0", Some(os), Some(arch));
            new_record.checksum = Some(Checksum::sha256("real_hash".into()));

            let mut lock = ProtoLock::default();
            lock.tools
                .insert(Id::raw("node"), vec![old_record, new_record]);
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            locker
                .update_spec_in_lockfile(
                    &UnresolvedVersionSpec::parse("^20").unwrap(),
                    &UnresolvedVersionSpec::parse("21.1.0").unwrap(),
                    &VersionSpec::parse("21.1.0").unwrap(),
                )
                .unwrap();

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();

            // The migrated record was merged into the existing one
            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].spec,
                Some(UnresolvedVersionSpec::parse("21.1.0").unwrap())
            );
            assert_eq!(
                records[0].checksum,
                Some(Checksum::sha256("real_hash".into()))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn keeps_checksum_when_version_unchanged() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            let mut record = make_record(
                "20.0.0",
                "^20",
                Some(SystemOS::default()),
                Some(SystemArch::default()),
            );
            record.checksum = Some(Checksum::sha256("keep_me".into()));

            let mut lock = ProtoLock::default();
            lock.tools.insert(Id::raw("node"), vec![record]);
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            // Same version, different spec
            locker
                .update_spec_in_lockfile(
                    &UnresolvedVersionSpec::parse("^20").unwrap(),
                    &UnresolvedVersionSpec::parse("20.0.0").unwrap(),
                    &VersionSpec::parse("20.0.0").unwrap(),
                )
                .unwrap();

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].spec,
                Some(UnresolvedVersionSpec::parse("20.0.0").unwrap())
            );
            assert_eq!(
                records[0].checksum,
                Some(Checksum::sha256("keep_me".into()))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_op_when_spec_not_found() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            let record = make_record(
                "18.0.0",
                "18.0.0",
                Some(SystemOS::default()),
                Some(SystemArch::default()),
            );

            let mut lock = ProtoLock::default();
            lock.tools.insert(Id::raw("node"), vec![record]);
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            locker
                .update_spec_in_lockfile(
                    &UnresolvedVersionSpec::parse("^20").unwrap(),
                    &UnresolvedVersionSpec::parse("21.1.0").unwrap(),
                    &VersionSpec::parse("21.1.0").unwrap(),
                )
                .unwrap();

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].spec,
                Some(UnresolvedVersionSpec::parse("18.0.0").unwrap())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_op_when_specs_are_equal() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            let mut record = make_record(
                "20.0.0",
                "20.0.0",
                Some(SystemOS::default()),
                Some(SystemArch::default()),
            );
            record.checksum = Some(Checksum::sha256("keep_me".into()));

            let mut lock = ProtoLock::default();
            lock.tools.insert(Id::raw("node"), vec![record]);
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            locker
                .update_spec_in_lockfile(
                    &UnresolvedVersionSpec::parse("20.0.0").unwrap(),
                    &UnresolvedVersionSpec::parse("20.0.0").unwrap(),
                    &VersionSpec::parse("20.0.0").unwrap(),
                )
                .unwrap();

            // The checksum should not have been wiped
            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].checksum,
                Some(Checksum::sha256("keep_me".into()))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_op_when_no_lockfile_config() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            locker
                .update_spec_in_lockfile(
                    &UnresolvedVersionSpec::parse("^20").unwrap(),
                    &UnresolvedVersionSpec::parse("21.1.0").unwrap(),
                    &VersionSpec::parse("21.1.0").unwrap(),
                )
                .unwrap();

            assert!(!sandbox.path().join(".protolock").exists());
        }
    }

    mod remove_spec {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_matching_records_across_os_arch() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            let mut lock = ProtoLock::default();
            lock.tools.insert(
                Id::raw("node"),
                vec![
                    make_record(
                        "20.0.0",
                        "^20",
                        Some(SystemOS::Linux),
                        Some(SystemArch::X64),
                    ),
                    make_record(
                        "20.0.0",
                        "^20",
                        Some(SystemOS::MacOS),
                        Some(SystemArch::Arm64),
                    ),
                    make_record(
                        "18.0.0",
                        "18.0.0",
                        Some(SystemOS::Linux),
                        Some(SystemArch::X64),
                    ),
                ],
            );
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            locker
                .remove_spec_from_lockfile(&UnresolvedVersionSpec::parse("^20").unwrap())
                .unwrap();

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].spec,
                Some(UnresolvedVersionSpec::parse("18.0.0").unwrap())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_tool_entry_when_all_records_removed() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            let mut lock = ProtoLock::default();
            lock.tools.insert(
                Id::raw("node"),
                vec![make_record(
                    "20.0.0",
                    "^20",
                    Some(SystemOS::default()),
                    Some(SystemArch::default()),
                )],
            );
            lock.tools.insert(
                Id::raw("bun"),
                vec![make_record(
                    "1.0.0",
                    "1.0.0",
                    Some(SystemOS::default()),
                    Some(SystemArch::default()),
                )],
            );
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            locker
                .remove_spec_from_lockfile(&UnresolvedVersionSpec::parse("^20").unwrap())
                .unwrap();

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();

            assert!(!lock.tools.contains_key(&Id::raw("node")));
            assert!(lock.tools.contains_key(&Id::raw("bun")));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_op_when_spec_not_found() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            let mut lock = ProtoLock::default();
            lock.tools.insert(
                Id::raw("node"),
                vec![make_record(
                    "18.0.0",
                    "18.0.0",
                    Some(SystemOS::default()),
                    Some(SystemArch::default()),
                )],
            );
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            locker
                .remove_spec_from_lockfile(&UnresolvedVersionSpec::parse("^20").unwrap())
                .unwrap();

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();

            assert_eq!(records.len(), 1);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_op_when_no_lockfile_config() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            locker
                .remove_spec_from_lockfile(&UnresolvedVersionSpec::parse("^20").unwrap())
                .unwrap();

            assert!(!sandbox.path().join(".protolock").exists());
        }
    }

    mod get_locked_versions {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_versions_for_current_os_arch() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            let other_os = if SystemOS::default() == SystemOS::Linux {
                SystemOS::MacOS
            } else {
                SystemOS::Linux
            };

            let mut lock = ProtoLock::default();
            lock.tools.insert(
                Id::raw("node"),
                vec![
                    // Current platform
                    make_record(
                        "20.0.0",
                        "^20",
                        Some(SystemOS::default()),
                        Some(SystemArch::default()),
                    ),
                    // Other platform
                    make_record("21.0.0", "^21", Some(other_os), Some(SystemArch::default())),
                    // Backwards compatible record without os/arch
                    make_record("18.0.0", "18.0.0", None, None),
                ],
            );
            lock.path = sandbox.path().join(".protolock");
            lock.save().unwrap();

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let versions = locker.get_locked_versions().unwrap();

            assert_eq!(versions.len(), 2);
            assert!(versions.contains(&VersionSpec::parse("18.0.0").unwrap()));
            assert!(versions.contains(&VersionSpec::parse("20.0.0").unwrap()));
            assert!(!versions.contains(&VersionSpec::parse("21.0.0").unwrap()));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_empty_when_no_lockfile() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            let versions = locker.get_locked_versions().unwrap();

            assert!(versions.is_empty());
        }
    }

    mod config_scoping {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_lock_when_tool_pinned_in_unlocked_nested_config() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");
            sandbox.create_file("nested/.prototools", "node = \"20.0.0\"");

            // Pre-create a lockfile in the locked root with a matching record
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

            let tool =
                create_tool_in_sandbox_at(sandbox.path(), &sandbox.path().join("nested")).await;
            let locker = Locker::new(&tool);

            // The nested config defines node, so the parent lock doesn't apply
            let spec = ToolSpec::parse("20.0.0").unwrap();
            let result = locker.resolve_locked_record(&spec).unwrap();

            assert!(result.is_none());

            // And inserting a record is a no-op
            let record = make_record("21.0.0", "21.0.0", Some(os), Some(arch));
            locker.insert_record_into_lockfile(&record).unwrap();

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].version,
                Some(VersionSpec::parse("20.0.0").unwrap())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn routes_to_the_config_that_pins_the_tool() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20.0.0\"\n\n[settings]\nlockfile = true",
            );
            sandbox.create_file("nested/.prototools", "bun = \"1.0.0\"");

            let tool =
                create_tool_in_sandbox_at(sandbox.path(), &sandbox.path().join("nested")).await;
            let locker = Locker::new(&tool);

            // Node is pinned in the locked root, so records are written there,
            // even when running from within the nested config's directory
            let record = make_record(
                "20.0.0",
                "20.0.0",
                Some(SystemOS::default()),
                Some(SystemArch::default()),
            );

            locker.insert_record_into_lockfile(&record).unwrap();

            assert!(!sandbox.path().join("nested/.protolock").exists());

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();

            assert_eq!(records.len(), 1);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_lock_for_adhoc_tool_when_closest_config_is_unlocked() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");
            sandbox.create_file("nested/.prototools", "bun = \"1.0.0\"");

            let tool =
                create_tool_in_sandbox_at(sandbox.path(), &sandbox.path().join("nested")).await;
            let locker = Locker::new(&tool);

            // Node isn't pinned anywhere, so it's owned by the closest
            // config (nested), which isn't locked
            let record = make_record(
                "20.0.0",
                "20.0.0",
                Some(SystemOS::default()),
                Some(SystemArch::default()),
            );

            locker.insert_record_into_lockfile(&record).unwrap();

            assert!(!sandbox.path().join(".protolock").exists());
            assert!(!sandbox.path().join("nested/.protolock").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn applies_lock_for_adhoc_tool_within_locked_config_scope() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");

            // No configs between the locked root and the working directory
            let tool =
                create_tool_in_sandbox_at(sandbox.path(), &sandbox.path().join("nested/deep"))
                    .await;
            let locker = Locker::new(&tool);

            let record = make_record(
                "20.0.0",
                "20.0.0",
                Some(SystemOS::default()),
                Some(SystemArch::default()),
            );

            locker.insert_record_into_lockfile(&record).unwrap();

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();

            assert!(lock.tools.contains_key(&Id::raw("node")));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_sibling_lockfiles_in_nested_configs() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20.0.0\"\n\n[settings]\nlockfile = true",
            );
            sandbox.create_file(
                "nested/.prototools",
                "node = \"22.0.0\"\n\n[settings]\nlockfile = true",
            );

            let tool =
                create_tool_in_sandbox_at(sandbox.path(), &sandbox.path().join("nested")).await;
            let locker = Locker::new(&tool);

            // The nested config defines node and is locked itself,
            // so records are written to its own lockfile
            let record = make_record(
                "22.0.0",
                "22.0.0",
                Some(SystemOS::default()),
                Some(SystemArch::default()),
            );

            locker.insert_record_into_lockfile(&record).unwrap();

            assert!(!sandbox.path().join(".protolock").exists());

            let lock = ProtoLock::load_from(sandbox.path().join("nested")).unwrap();
            let records = lock.tools.get(&Id::raw("node")).unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].version,
                Some(VersionSpec::parse("22.0.0").unwrap())
            );
        }
    }

    mod env_scoping {
        use super::*;

        fn create_lock(path: &Path, version: &str) {
            let mut lock = ProtoLock::default();
            lock.tools
                .entry(Id::raw("node"))
                .or_default()
                .push(make_record(
                    version,
                    version,
                    Some(SystemOS::default()),
                    Some(SystemArch::default()),
                ));
            lock.path = path.to_path_buf();
            lock.save().unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn routes_to_the_env_lockfile_when_pinned_in_env_config() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20.0.0\"\n\n[settings]\nlockfile = true",
            );
            sandbox.create_file(".prototools.production", "node = \"22.0.0\"");

            create_lock(&sandbox.path().join(".protolock"), "20.0.0");
            create_lock(&sandbox.path().join(".protolock.production"), "22.0.0");

            let tool =
                create_tool_in_sandbox_with_env(sandbox.path(), sandbox.path(), Some("production"))
                    .await;
            let locker = Locker::new(&tool);

            // Resolves from the env lockfile, and never the base lockfile
            let record = locker
                .resolve_locked_record(&ToolSpec::parse("22.0.0").unwrap())
                .unwrap()
                .unwrap();

            assert_eq!(record.version, Some(VersionSpec::parse("22.0.0").unwrap()));

            assert!(
                locker
                    .resolve_locked_record(&ToolSpec::parse("20.0.0").unwrap())
                    .unwrap()
                    .is_none()
            );

            // Records are written to the env lockfile
            let record = make_record(
                "22.1.0",
                "22.1.0",
                Some(SystemOS::default()),
                Some(SystemArch::default()),
            );

            locker.insert_record_into_lockfile(&record).unwrap();

            let env_lock = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();
            let records = env_lock.tools.get(&Id::raw("node")).unwrap();

            assert_eq!(records.len(), 2);
            assert_eq!(
                records[1].version,
                Some(VersionSpec::parse("22.1.0").unwrap())
            );

            // While the base lockfile is untouched
            let base_lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = base_lock.tools.get(&Id::raw("node")).unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].version,
                Some(VersionSpec::parse("20.0.0").unwrap())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn routes_to_the_base_lockfile_when_not_pinned_in_env_config() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20.0.0\"\n\n[settings]\nlockfile = true",
            );
            sandbox.create_file(".prototools.production", "bun = \"1.0.0\"");

            create_lock(&sandbox.path().join(".protolock"), "20.0.0");

            let tool =
                create_tool_in_sandbox_with_env(sandbox.path(), sandbox.path(), Some("production"))
                    .await;
            let locker = Locker::new(&tool);

            let record = locker
                .resolve_locked_record(&ToolSpec::parse("20.0.0").unwrap())
                .unwrap()
                .unwrap();

            assert_eq!(record.version, Some(VersionSpec::parse("20.0.0").unwrap()));

            let record = make_record(
                "20.1.0",
                "20.1.0",
                Some(SystemOS::default()),
                Some(SystemArch::default()),
            );

            locker.insert_record_into_lockfile(&record).unwrap();

            let base_lock = ProtoLock::load_from(sandbox.path()).unwrap();

            assert_eq!(base_lock.tools.get(&Id::raw("node")).unwrap().len(), 2);

            // The env config doesn't define node, so its lockfile is never created
            assert!(!sandbox.path().join(".protolock.production").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_env_lockfile_when_env_not_active() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20.0.0\"\n\n[settings]\nlockfile = true",
            );
            sandbox.create_file(".prototools.production", "node = \"22.0.0\"");

            create_lock(&sandbox.path().join(".protolock.production"), "22.0.0");

            let tool = create_tool_in_sandbox(sandbox.path()).await;
            let locker = Locker::new(&tool);

            assert!(
                locker
                    .resolve_locked_record(&ToolSpec::parse("22.0.0").unwrap())
                    .unwrap()
                    .is_none()
            );

            let record = make_record(
                "20.0.0",
                "20.0.0",
                Some(SystemOS::default()),
                Some(SystemArch::default()),
            );

            locker.insert_record_into_lockfile(&record).unwrap();

            let base_lock = ProtoLock::load_from(sandbox.path()).unwrap();

            assert!(base_lock.tools.contains_key(&Id::raw("node")));

            // Inactive env lockfiles are never touched
            let env_lock = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();
            let records = env_lock.tools.get(&Id::raw("node")).unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].version,
                Some(VersionSpec::parse("22.0.0").unwrap())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_from_the_env_lockfile_only() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20.0.0\"\n\n[settings]\nlockfile = true",
            );
            sandbox.create_file(".prototools.production", "node = \"22.0.0\"");

            create_lock(&sandbox.path().join(".protolock"), "20.0.0");
            create_lock(&sandbox.path().join(".protolock.production"), "22.0.0");

            let tool =
                create_tool_in_sandbox_with_env(sandbox.path(), sandbox.path(), Some("production"))
                    .await;

            Locker::new(&tool).remove_from_lockfile().unwrap();

            // Empty lockfiles are removed
            assert!(!sandbox.path().join(".protolock.production").exists());

            let base_lock = ProtoLock::load_from(sandbox.path()).unwrap();

            assert!(base_lock.tools.contains_key(&Id::raw("node")));
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

    mod prune_orphaned_records {
        use super::*;
        use std::collections::BTreeMap;

        fn create_env_in_sandbox(sandbox_path: &Path) -> ProtoEnvironment {
            create_env_in_sandbox_with_mode(sandbox_path, None)
        }

        fn create_env_in_sandbox_with_mode(
            sandbox_path: &Path,
            env_mode: Option<&str>,
        ) -> ProtoEnvironment {
            let mut proto = ProtoEnvironment::new_testing(sandbox_path).unwrap();
            proto.working_dir = sandbox_path.to_path_buf();
            // Don't inherit `PROTO_ENV` from the test process
            proto.env_mode = env_mode.map(|env| env.to_owned());
            proto
        }

        fn seed_lockfile(dir: &Path, records: &[(&str, &str, Option<&str>)]) {
            seed_lockfile_at(&dir.join(".protolock"), records);
        }

        fn seed_lockfile_at(lock_path: &Path, records: &[(&str, &str, Option<&str>)]) {
            let mut lock = ProtoLock::default();

            for (id, spec, backend) in records {
                lock.tools.entry(Id::raw(id)).or_default().push(LockRecord {
                    spec: Some(UnresolvedVersionSpec::parse(spec).unwrap()),
                    backend: backend.map(Id::raw),
                    os: Some(SystemOS::default()),
                    arch: Some(SystemArch::default()),
                    ..Default::default()
                });
            }

            lock.path = lock_path.to_path_buf();
            lock.save().unwrap();
        }

        fn installing(entries: &[(&str, &str)]) -> BTreeMap<ToolContext, UnresolvedVersionSpec> {
            entries
                .iter()
                .map(|(context, spec)| {
                    (
                        ToolContext::parse(context).unwrap(),
                        UnresolvedVersionSpec::parse(spec).unwrap(),
                    )
                })
                .collect()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prunes_stale_records_for_configured_tools() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20\"\n\n[settings]\nlockfile = true",
            );
            seed_lockfile(
                sandbox.path(),
                &[
                    ("node", "20", None),
                    ("node", "18", None),
                    ("bun", "1", None),
                ],
            );

            let proto = create_env_in_sandbox(sandbox.path());
            let pruned = Locker::prune_orphaned_records(&proto, &BTreeMap::default()).unwrap();

            assert_eq!(pruned, 1);

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let node_records = lock.tools.get("node").unwrap();

            assert_eq!(node_records.len(), 1);
            assert_eq!(
                node_records[0].spec.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("20").unwrap()
            );

            // Not configured, so ad-hoc records are kept
            assert_eq!(lock.tools.get("bun").unwrap().len(), 1);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn keeps_records_for_specs_being_installed() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20\"\n\n[settings]\nlockfile = true",
            );
            seed_lockfile(sandbox.path(), &[("node", "21", None)]);

            let proto = create_env_in_sandbox(sandbox.path());
            let pruned =
                Locker::prune_orphaned_records(&proto, &installing(&[("node", "21")])).unwrap();

            assert_eq!(pruned, 0);

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();

            assert_eq!(lock.tools.get("node").unwrap().len(), 1);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn installing_specs_dont_verify_unconfigured_tools() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20\"\n\n[settings]\nlockfile = true",
            );
            seed_lockfile(
                sandbox.path(),
                &[("bun", "1.0.0", None), ("bun", "1.1.0", None)],
            );

            let proto = create_env_in_sandbox(sandbox.path());

            // Installing an ad-hoc tool must not cause its other
            // ad-hoc records to be treated as orphans
            let pruned =
                Locker::prune_orphaned_records(&proto, &installing(&[("bun", "1.1.0")])).unwrap();

            assert_eq!(pruned, 0);

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();

            assert_eq!(lock.tools.get("bun").unwrap().len(), 2);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_when_no_versions_configured() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "[settings]\nlockfile = true");
            seed_lockfile(
                sandbox.path(),
                &[("node", "20", None), ("node", "18", None)],
            );

            let proto = create_env_in_sandbox(sandbox.path());
            let pruned = Locker::prune_orphaned_records(&proto, &BTreeMap::default()).unwrap();

            assert_eq!(pruned, 0);

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();

            assert_eq!(lock.tools.get("node").unwrap().len(), 2);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_lockfile_when_all_records_are_pruned() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20\"\n\n[settings]\nlockfile = true",
            );
            seed_lockfile(sandbox.path(), &[("node", "18", None)]);

            let proto = create_env_in_sandbox(sandbox.path());
            let pruned = Locker::prune_orphaned_records(&proto, &BTreeMap::default()).unwrap();

            assert_eq!(pruned, 1);
            assert!(!sandbox.path().join(".protolock").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prunes_each_lockfile_against_its_own_configs() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20\"\n\n[settings]\nlockfile = true",
            );
            sandbox.create_file(
                "nested/.prototools",
                "node = \"21\"\n\n[settings]\nlockfile = true",
            );
            seed_lockfile(
                sandbox.path(),
                &[("node", "20", None), ("node", "19", None)],
            );
            seed_lockfile(
                &sandbox.path().join("nested"),
                &[("node", "21", None), ("node", "20", None)],
            );

            let mut proto = create_env_in_sandbox(sandbox.path());
            proto.working_dir = sandbox.path().join("nested");

            let pruned = Locker::prune_orphaned_records(&proto, &BTreeMap::default()).unwrap();

            assert_eq!(pruned, 2);

            let root_lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let root_records = root_lock.tools.get("node").unwrap();

            assert_eq!(root_records.len(), 1);
            assert_eq!(
                root_records[0].spec.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("20").unwrap()
            );

            let nested_lock = ProtoLock::load_from(sandbox.path().join("nested")).unwrap();
            let nested_records = nested_lock.tools.get("node").unwrap();

            assert_eq!(nested_records.len(), 1);
            assert_eq!(
                nested_records[0].spec.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("21").unwrap()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prunes_the_active_env_lockfile_against_its_own_config() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20\"\n\n[settings]\nlockfile = true",
            );
            sandbox.create_file(".prototools.dev", "node = \"22\"");

            seed_lockfile(
                sandbox.path(),
                &[("node", "20", None), ("node", "19", None)],
            );
            seed_lockfile_at(
                &sandbox.path().join(".protolock.dev"),
                &[("node", "22", None), ("node", "21", None)],
            );

            let proto = create_env_in_sandbox_with_mode(sandbox.path(), Some("dev"));
            let pruned = Locker::prune_orphaned_records(&proto, &BTreeMap::default()).unwrap();

            assert_eq!(pruned, 2);

            // Each lockfile is scoped to the config that enabled it, so the
            // base spec doesn't protect records in the env lockfile
            let base_lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let base_records = base_lock.tools.get("node").unwrap();

            assert_eq!(base_records.len(), 1);
            assert_eq!(
                base_records[0].spec.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("20").unwrap()
            );

            let env_lock = ProtoLock::load(sandbox.path().join(".protolock.dev")).unwrap();
            let env_records = env_lock.tools.get("node").unwrap();

            assert_eq!(env_records.len(), 1);
            assert_eq!(
                env_records[0].spec.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("22").unwrap()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prunes_env_lockfiles_that_are_not_active() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20\"\n\n[settings]\nlockfile = true",
            );
            // Neither environment is active, but both own a lockfile
            sandbox.create_file(".prototools.dev", "node = \"22\"");
            sandbox.create_file(".prototools.prod", "node = \"18\"");

            seed_lockfile(sandbox.path(), &[("node", "20", None)]);
            seed_lockfile_at(
                &sandbox.path().join(".protolock.dev"),
                &[("node", "22", None), ("node", "21", None)],
            );
            seed_lockfile_at(
                &sandbox.path().join(".protolock.prod"),
                &[("node", "17", None)],
            );

            let proto = create_env_in_sandbox(sandbox.path());
            let pruned = Locker::prune_orphaned_records(&proto, &BTreeMap::default()).unwrap();

            assert_eq!(pruned, 2);

            assert_eq!(
                ProtoLock::load_from(sandbox.path())
                    .unwrap()
                    .tools
                    .get("node")
                    .unwrap()
                    .len(),
                1
            );

            let dev_lock = ProtoLock::load(sandbox.path().join(".protolock.dev")).unwrap();
            let dev_records = dev_lock.tools.get("node").unwrap();

            assert_eq!(dev_records.len(), 1);
            assert_eq!(
                dev_records[0].spec.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("22").unwrap()
            );

            // Every record was orphaned, so the lockfile was removed
            assert!(!sandbox.path().join(".protolock.prod").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_prune_env_lockfiles_when_not_enabled() {
            let sandbox = create_empty_sandbox();
            // Lockfiles are not enabled, so neither config owns one
            sandbox.create_file(".prototools", "node = \"20\"");
            sandbox.create_file(".prototools.dev", "node = \"22\"");

            seed_lockfile_at(
                &sandbox.path().join(".protolock.dev"),
                &[("node", "21", None)],
            );

            let proto = create_env_in_sandbox(sandbox.path());
            let pruned = Locker::prune_orphaned_records(&proto, &BTreeMap::default()).unwrap();

            assert_eq!(pruned, 0);
            assert!(sandbox.path().join(".protolock.dev").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prunes_env_lockfiles_that_enable_the_setting_themselves() {
            let sandbox = create_empty_sandbox();
            // Only the env config enables a lockfile
            sandbox.create_file(".prototools", "node = \"20\"");
            sandbox.create_file(
                ".prototools.dev",
                "node = \"22\"\n\n[settings]\nlockfile = true",
            );

            seed_lockfile(sandbox.path(), &[("node", "19", None)]);
            seed_lockfile_at(
                &sandbox.path().join(".protolock.dev"),
                &[("node", "21", None)],
            );

            let proto = create_env_in_sandbox(sandbox.path());
            let pruned = Locker::prune_orphaned_records(&proto, &BTreeMap::default()).unwrap();

            assert_eq!(pruned, 1);

            // Every record was orphaned, so the lockfile was removed
            assert!(!sandbox.path().join(".protolock.dev").exists());

            // And the base lockfile was removed when configs were loaded,
            // since its config never enabled the setting
            assert!(!sandbox.path().join(".protolock").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_config_like_files_without_a_lockfile() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20\"\n\n[settings]\nlockfile = true",
            );
            // Not a real config, and has no lockfile to maintain,
            // so it's never loaded and can't disrupt pruning
            sandbox.create_file(".prototools.bak", "node = [");

            seed_lockfile(
                sandbox.path(),
                &[("node", "20", None), ("node", "18", None)],
            );

            let proto = create_env_in_sandbox(sandbox.path());
            let pruned = Locker::prune_orphaned_records(&proto, &BTreeMap::default()).unwrap();

            assert_eq!(pruned, 1);
            assert_eq!(
                ProtoLock::load_from(sandbox.path())
                    .unwrap()
                    .tools
                    .get("node")
                    .unwrap()
                    .len(),
                1
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_pruning_when_env_config_is_unloadable() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(
                ".prototools",
                "node = \"20\"\n\n[settings]\nlockfile = true",
            );
            // Invalid TOML, and owns a lockfile, so the specs that
            // protect its records cannot be determined
            sandbox.create_file(".prototools.broken", "node = [");

            seed_lockfile(sandbox.path(), &[("node", "18", None)]);
            seed_lockfile_at(
                &sandbox.path().join(".protolock.broken"),
                &[("node", "17", None)],
            );

            let proto = create_env_in_sandbox(sandbox.path());
            let pruned = Locker::prune_orphaned_records(&proto, &BTreeMap::default()).unwrap();

            // The base lockfile is still pruned, as it's loaded and
            // doesn't depend on the broken config
            assert_eq!(pruned, 1);
            assert!(!sandbox.path().join(".protolock").exists());
            assert!(sandbox.path().join(".protolock.broken").exists());
        }
    }
}
