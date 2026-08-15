use proto_core::{Id, LockRecord, ProtoLock};
use proto_pdk_api::ToolLockOptions;
use starbase_sandbox::create_empty_sandbox;
use system_env::{SystemArch, SystemOS};
use version_spec::{UnresolvedVersionSpec, VersionSpec};

mod lockfile {
    use super::*;

    mod lock_record_matching {
        use super::*;

        fn default_options() -> ToolLockOptions {
            ToolLockOptions::default()
        }

        fn record_with(
            spec: Option<&str>,
            backend: Option<&str>,
            os: Option<SystemOS>,
            arch: Option<SystemArch>,
        ) -> LockRecord {
            LockRecord {
                spec: spec.map(|s| UnresolvedVersionSpec::parse(s).unwrap()),
                backend: backend.map(Id::raw),
                os,
                arch,
                ..Default::default()
            }
        }

        #[test]
        fn matches_same_spec_and_backend() {
            let a = record_with(Some("1.2.3"), None, None, None);
            let b = record_with(Some("1.2.3"), None, None, None);

            assert!(a.is_match(&b, &default_options()));
        }

        #[test]
        fn no_match_different_spec() {
            let a = record_with(Some("1.2.3"), None, None, None);
            let b = record_with(Some("2.0.0"), None, None, None);

            assert!(!a.is_match(&b, &default_options()));
        }

        #[test]
        fn no_match_different_backend() {
            let a = record_with(Some("1.2.3"), Some("asdf"), None, None);
            let b = record_with(Some("1.2.3"), Some("proto"), None, None);

            assert!(!a.is_match(&b, &default_options()));
        }

        #[test]
        fn matches_when_record_os_arch_none_backwards_compat() {
            // Record in lockfile has no os/arch (old format) — should match any os/arch query
            let record = record_with(Some("1.2.3"), None, None, None);
            let query = record_with(
                Some("1.2.3"),
                None,
                Some(SystemOS::Linux),
                Some(SystemArch::X64),
            );

            assert!(record.is_match(&query, &default_options()));
        }

        #[test]
        fn no_match_different_os() {
            let record = record_with(Some("1.2.3"), None, Some(SystemOS::Linux), None);
            let query = record_with(Some("1.2.3"), None, Some(SystemOS::MacOS), None);

            assert!(!record.is_match(&query, &default_options()));
        }

        #[test]
        fn no_match_different_arch() {
            let record = record_with(Some("1.2.3"), None, None, Some(SystemArch::X64));
            let query = record_with(Some("1.2.3"), None, None, Some(SystemArch::Arm64));

            assert!(!record.is_match(&query, &default_options()));
        }

        #[test]
        fn matches_same_os_and_arch() {
            let record = record_with(
                Some("1.2.3"),
                None,
                Some(SystemOS::Linux),
                Some(SystemArch::X64),
            );
            let query = record_with(
                Some("1.2.3"),
                None,
                Some(SystemOS::Linux),
                Some(SystemArch::X64),
            );

            assert!(record.is_match(&query, &default_options()));
        }

        #[test]
        fn ignores_os_arch_when_option_set() {
            // When ignore_os_arch is true AND record has no os/arch, it should match
            let record = record_with(Some("1.2.3"), None, None, None);
            let query = record_with(
                Some("1.2.3"),
                None,
                Some(SystemOS::Linux),
                Some(SystemArch::X64),
            );
            let options = ToolLockOptions {
                ignore_os_arch: true,
                ..Default::default()
            };

            assert!(record.is_match(&query, &options));
        }

        #[test]
        fn no_match_ignore_os_arch_but_record_has_os_arch() {
            // When ignore_os_arch is true but the record HAS os/arch,
            // it should NOT match (old records with os/arch are skipped)
            let record = record_with(
                Some("1.2.3"),
                None,
                Some(SystemOS::Linux),
                Some(SystemArch::X64),
            );
            let query = record_with(
                Some("1.2.3"),
                None,
                Some(SystemOS::Linux),
                Some(SystemArch::X64),
            );
            let options = ToolLockOptions {
                ignore_os_arch: true,
                ..Default::default()
            };

            assert!(!record.is_match(&query, &options));
        }

        #[test]
        fn matches_with_backend_and_spec_both_none() {
            let a = LockRecord::default();
            let b = LockRecord::default();

            assert!(a.is_match(&b, &default_options()));
        }
    }

    mod lock_record_conversions {
        use super::*;

        #[test]
        fn for_manifest_strips_spec_and_version() {
            let record = LockRecord {
                spec: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                version: Some(VersionSpec::parse("1.2.3").unwrap()),
                os: Some(SystemOS::Linux),
                arch: Some(SystemArch::X64),
                source: Some("https://example.com/file.tar.gz".into()),
                ..Default::default()
            };

            let manifest_record = record.for_manifest();

            assert!(manifest_record.spec.is_none());
            assert!(manifest_record.version.is_none());
            // Other fields preserved
            assert_eq!(manifest_record.os, Some(SystemOS::Linux));
            assert_eq!(manifest_record.arch, Some(SystemArch::X64));
            assert_eq!(
                manifest_record.source,
                Some("https://example.com/file.tar.gz".into())
            );
        }

        #[test]
        fn for_lockfile_strips_source() {
            let record = LockRecord {
                spec: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                version: Some(VersionSpec::parse("1.2.3").unwrap()),
                source: Some("https://example.com/file.tar.gz".into()),
                ..Default::default()
            };

            let lockfile_record = record.for_lockfile();

            assert!(lockfile_record.source.is_none());
            // Other fields preserved
            assert!(lockfile_record.spec.is_some());
            assert!(lockfile_record.version.is_some());
        }
    }

    mod proto_lock_io {
        use super::*;

        #[test]
        fn load_from_nonexistent_creates_default() {
            let sandbox = create_empty_sandbox();
            let lock = ProtoLock::load_from(sandbox.path()).unwrap();

            assert!(lock.tools.is_empty());
        }

        #[test]
        fn save_and_load_roundtrip() {
            let sandbox = create_empty_sandbox();

            let mut lock = ProtoLock::load_from(sandbox.path()).unwrap();

            let record = LockRecord {
                spec: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                version: Some(VersionSpec::parse("1.2.3").unwrap()),
                os: Some(SystemOS::Linux),
                arch: Some(SystemArch::X64),
                ..Default::default()
            };

            lock.tools
                .entry(Id::raw("node"))
                .or_default()
                .push(record.clone());

            lock.save().unwrap();

            // Reload
            let loaded = ProtoLock::load_from(sandbox.path()).unwrap();

            assert_eq!(loaded.tools.len(), 1);
            assert!(loaded.tools.contains_key(&Id::raw("node")));

            let records = loaded.tools.get(&Id::raw("node")).unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].spec, record.spec);
            assert_eq!(records[0].version, record.version);
            assert_eq!(records[0].os, record.os);
            assert_eq!(records[0].arch, record.arch);
        }

        #[test]
        fn save_removes_file_when_empty() {
            let sandbox = create_empty_sandbox();

            // Create a lockfile with content first
            let mut lock = ProtoLock::load_from(sandbox.path()).unwrap();
            lock.tools
                .entry(Id::raw("node"))
                .or_default()
                .push(LockRecord {
                    version: Some(VersionSpec::parse("1.0.0").unwrap()),
                    ..Default::default()
                });
            lock.save().unwrap();

            let lock_path = sandbox.path().join(".protolock");
            assert!(lock_path.exists());

            // Now save an empty lock
            let empty_lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let empty = ProtoLock {
                tools: Default::default(),
                path: empty_lock.path,
            };
            empty.save().unwrap();

            // File should be removed
            assert!(!lock_path.exists());
        }

        #[test]
        fn sort_records_orders_by_spec_then_backend() {
            let mut lock = ProtoLock::default();

            let records = vec![
                LockRecord {
                    spec: Some(UnresolvedVersionSpec::parse("2.0.0").unwrap()),
                    backend: Some(Id::raw("asdf")),
                    ..Default::default()
                },
                LockRecord {
                    spec: Some(UnresolvedVersionSpec::parse("1.0.0").unwrap()),
                    backend: Some(Id::raw("proto")),
                    ..Default::default()
                },
                LockRecord {
                    spec: Some(UnresolvedVersionSpec::parse("1.0.0").unwrap()),
                    backend: Some(Id::raw("asdf")),
                    ..Default::default()
                },
            ];

            lock.tools.insert(Id::raw("node"), records);
            lock.sort_records();

            let sorted = lock.tools.get(&Id::raw("node")).unwrap();
            // Should be sorted by spec first, then backend
            assert_eq!(
                sorted[0].spec,
                Some(UnresolvedVersionSpec::parse("1.0.0").unwrap())
            );
            assert_eq!(sorted[0].backend, Some(Id::raw("asdf")));
            assert_eq!(
                sorted[1].spec,
                Some(UnresolvedVersionSpec::parse("1.0.0").unwrap())
            );
            assert_eq!(sorted[1].backend, Some(Id::raw("proto")));
            assert_eq!(
                sorted[2].spec,
                Some(UnresolvedVersionSpec::parse("2.0.0").unwrap())
            );
        }

        #[test]
        fn load_resolves_path_with_and_without_filename() {
            let sandbox = create_empty_sandbox();

            // Load with directory path
            let lock1 = ProtoLock::load_from(sandbox.path()).unwrap();
            assert!(lock1.path.ends_with(".protolock"));

            // Load with explicit file path
            let lock2 = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            assert!(lock2.path.ends_with(".protolock"));
        }
    }

    mod prune_orphaned_records {
        use super::*;
        use proto_core::ToolContext;
        use std::collections::{BTreeMap, BTreeSet};

        fn record_with(spec: Option<&str>, backend: Option<&str>) -> LockRecord {
            LockRecord {
                spec: spec.map(|s| UnresolvedVersionSpec::parse(s).unwrap()),
                backend: backend.map(Id::raw),
                os: Some(SystemOS::default()),
                arch: Some(SystemArch::default()),
                ..Default::default()
            }
        }

        fn config_specs(
            entries: &[(&str, &[&str])],
        ) -> BTreeMap<ToolContext, BTreeSet<UnresolvedVersionSpec>> {
            entries
                .iter()
                .map(|(context, specs)| {
                    (
                        ToolContext::parse(context).unwrap(),
                        specs
                            .iter()
                            .map(|spec| UnresolvedVersionSpec::parse(spec).unwrap())
                            .collect(),
                    )
                })
                .collect()
        }

        #[test]
        fn prunes_records_not_matching_configured_specs() {
            let mut lock = ProtoLock::default();
            lock.tools.insert(
                Id::raw("node"),
                vec![record_with(Some("20"), None), record_with(Some("18"), None)],
            );

            let pruned = lock.prune_orphaned_records(&config_specs(&[("node", &["20"])]));

            assert_eq!(pruned, 1);

            let records = lock.tools.get("node").unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].spec.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("20").unwrap()
            );
        }

        #[test]
        fn keeps_records_for_unconfigured_tools() {
            let mut lock = ProtoLock::default();
            lock.tools.insert(
                Id::raw("bun"),
                vec![
                    record_with(Some("1.0.0"), None),
                    record_with(Some("1.1.0"), None),
                ],
            );

            let pruned = lock.prune_orphaned_records(&config_specs(&[("node", &["20"])]));

            assert_eq!(pruned, 0);
            assert_eq!(lock.tools.get("bun").unwrap().len(), 2);
        }

        #[test]
        fn keeps_records_without_a_spec() {
            let mut lock = ProtoLock::default();
            lock.tools
                .insert(Id::raw("node"), vec![record_with(None, None)]);

            let pruned = lock.prune_orphaned_records(&config_specs(&[("node", &["20"])]));

            assert_eq!(pruned, 0);
            assert_eq!(lock.tools.get("node").unwrap().len(), 1);
        }

        #[test]
        fn keeps_records_with_a_different_backend() {
            let mut lock = ProtoLock::default();
            lock.tools.insert(
                Id::raw("node"),
                vec![
                    // Configured with a backend, so the backendless
                    // record is an ad-hoc install
                    record_with(Some("18"), None),
                    record_with(Some("18"), Some("asdf")),
                ],
            );

            let pruned = lock.prune_orphaned_records(&config_specs(&[("asdf:node", &["20"])]));

            assert_eq!(pruned, 1);

            let records = lock.tools.get("node").unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(records[0].backend, None);
        }

        #[test]
        fn prunes_across_os_and_arch() {
            let mut lock = ProtoLock::default();
            lock.tools.insert(
                Id::raw("node"),
                vec![
                    LockRecord {
                        spec: Some(UnresolvedVersionSpec::parse("18").unwrap()),
                        os: Some(SystemOS::Linux),
                        arch: Some(SystemArch::Arm64),
                        ..Default::default()
                    },
                    LockRecord {
                        spec: Some(UnresolvedVersionSpec::parse("18").unwrap()),
                        os: Some(SystemOS::Windows),
                        arch: Some(SystemArch::X64),
                        ..Default::default()
                    },
                    LockRecord {
                        spec: Some(UnresolvedVersionSpec::parse("20").unwrap()),
                        os: Some(SystemOS::Linux),
                        arch: Some(SystemArch::Arm64),
                        ..Default::default()
                    },
                ],
            );

            let pruned = lock.prune_orphaned_records(&config_specs(&[("node", &["20"])]));

            assert_eq!(pruned, 2);
            assert_eq!(lock.tools.get("node").unwrap().len(), 1);
        }

        #[test]
        fn removes_tool_when_all_records_are_pruned() {
            let mut lock = ProtoLock::default();
            lock.tools
                .insert(Id::raw("node"), vec![record_with(Some("18"), None)]);
            lock.tools
                .insert(Id::raw("bun"), vec![record_with(Some("1.0.0"), None)]);

            let pruned = lock.prune_orphaned_records(&config_specs(&[("node", &["20"])]));

            assert_eq!(pruned, 1);
            assert!(!lock.tools.contains_key("node"));
            assert!(lock.tools.contains_key("bun"));
        }

        #[test]
        fn returns_zero_when_nothing_is_orphaned() {
            let mut lock = ProtoLock::default();
            lock.tools
                .insert(Id::raw("node"), vec![record_with(Some("20"), None)]);

            let pruned = lock.prune_orphaned_records(&config_specs(&[("node", &["20", "18"])]));

            assert_eq!(pruned, 0);
            assert_eq!(lock.tools.get("node").unwrap().len(), 1);
        }
    }
}
