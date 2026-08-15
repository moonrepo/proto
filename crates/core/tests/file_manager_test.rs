use proto_core::{Id, LockRecord, ProtoFileManager, ToolContext};
use starbase_sandbox::create_empty_sandbox;
use std::collections::BTreeMap;
use std::path::PathBuf;
use version_spec::UnresolvedVersionSpec;
use warpgate::{FileLocator, PluginLocator};

fn get_locked_config_path(manager: &ProtoFileManager, id: &str) -> Option<PathBuf> {
    manager
        .get_locked_config(&ToolContext::parse(id).unwrap())
        .map(|file| file.path.clone())
}

mod file_manager {
    use super::*;

    #[test]
    fn merges_traversing_upwards() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(
            "one/two/three/.prototools",
            r#"
node = "1.2.3"

[plugins.tools]
node = "file://./node.toml"
"#,
        );

        sandbox.create_file(
            "one/two/.prototools",
            r#"
[plugins.tools]
bun = "file://../bun.wasm"
"#,
        );

        sandbox.create_file(
            "one/.prototools",
            r#"
bun = "4.5.6"

[plugins.tools]
node = "file://../node.toml"
"#,
        );

        sandbox.create_file(
            ".prototools",
            r#"
node = "7.8.9"
deno = "7.8.9"
"#,
        );

        let manager = ProtoFileManager::load(
            sandbox.path().join("one/two/three"),
            Some(sandbox.path().parent().unwrap()),
            None,
        )
        .unwrap();
        let config = manager.get_merged_config().unwrap();

        assert_eq!(
            config.versions,
            BTreeMap::from_iter([
                (
                    ToolContext::parse("node").unwrap(),
                    UnresolvedVersionSpec::parse("1.2.3").unwrap().into()
                ),
                (
                    ToolContext::parse("bun").unwrap(),
                    UnresolvedVersionSpec::parse("4.5.6").unwrap().into()
                ),
                (
                    ToolContext::parse("deno").unwrap(),
                    UnresolvedVersionSpec::parse("7.8.9").unwrap().into()
                ),
            ])
        );

        assert_eq!(
            config.plugins.tools.get("node").unwrap(),
            &PluginLocator::File(Box::new(FileLocator {
                file: "file://./node.toml".into(),
                path: Some(sandbox.path().join("one/two/three/./node.toml"))
            }))
        );

        assert_eq!(
            config.plugins.tools.get("bun").unwrap(),
            &PluginLocator::File(Box::new(FileLocator {
                file: "file://../bun.wasm".into(),
                path: Some(sandbox.path().join("one/two/../bun.wasm"))
            }))
        );
    }

    #[test]
    fn merges_traversing_upwards_without_global() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(
            "one/two/three/.prototools",
            r#"
node = "1.2.3"
"#,
        );

        sandbox.create_file(
            ".prototools",
            r#"
node = "7.8.9"
deno = "7.8.9"
"#,
        );

        sandbox.create_file(
            ".proto/.prototools",
            r#"
bun = "1.2.3"
"#,
        );

        let manager = ProtoFileManager::load(
            sandbox.path().join("one/two/three"),
            Some(sandbox.path().parent().unwrap()),
            None,
        )
        .unwrap();
        let config = manager.get_merged_config_without_global().unwrap();

        assert_eq!(
            config.versions,
            BTreeMap::from_iter([
                (
                    ToolContext::parse("node").unwrap(),
                    UnresolvedVersionSpec::parse("1.2.3").unwrap().into()
                ),
                (
                    ToolContext::parse("deno").unwrap(),
                    UnresolvedVersionSpec::parse("7.8.9").unwrap().into()
                ),
            ])
        );
    }

    #[test]
    fn merges_local_only() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(
            "one/two/three/.prototools",
            r#"
node = "1.2.3"
"#,
        );

        sandbox.create_file(
            ".prototools",
            r#"
node = "7.8.9"
deno = "7.8.9"
"#,
        );

        sandbox.create_file(
            ".proto/.prototools",
            r#"
bun = "1.2.3"
"#,
        );

        let manager = ProtoFileManager::load(
            sandbox.path().join("one/two/three"),
            Some(sandbox.path().parent().unwrap()),
            None,
        )
        .unwrap();
        let config = manager
            .get_local_config(&sandbox.path().join("one/two/three"))
            .unwrap();

        assert_eq!(
            config.versions,
            BTreeMap::from_iter([(
                ToolContext::parse("node").unwrap(),
                UnresolvedVersionSpec::parse("1.2.3").unwrap().into()
            )])
        );
    }

    #[test]
    fn supports_env_mode() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(
            ".prototools.production",
            r#"
node = "1.2.3"
"#,
        );

        sandbox.create_file(
            ".prototools",
            r#"
node = "7.8.9"
deno = "7.8.9"
"#,
        );

        let manager = ProtoFileManager::load(
            sandbox.path(),
            Some(sandbox.path().parent().unwrap()),
            Some(&"production".to_owned()),
        )
        .unwrap();
        let config = manager.get_local_config(sandbox.path()).unwrap();

        assert_eq!(
            config.versions,
            BTreeMap::from_iter([
                (
                    ToolContext::parse("node").unwrap(),
                    UnresolvedVersionSpec::parse("1.2.3").unwrap().into()
                ),
                (
                    ToolContext::parse("deno").unwrap(),
                    UnresolvedVersionSpec::parse("7.8.9").unwrap().into()
                ),
            ])
        );
    }

    #[test]
    fn ignores_env_file_when_mode_not_defined() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(
            ".prototools.production",
            r#"
node = "1.2.3"
"#,
        );

        sandbox.create_file(
            ".prototools",
            r#"
node = "7.8.9"
deno = "7.8.9"
"#,
        );

        let manager =
            ProtoFileManager::load(sandbox.path(), Some(sandbox.path().parent().unwrap()), None)
                .unwrap();
        let config = manager.get_local_config(sandbox.path()).unwrap();

        assert_eq!(
            config.versions,
            BTreeMap::from_iter([
                (
                    ToolContext::parse("node").unwrap(),
                    UnresolvedVersionSpec::parse("7.8.9").unwrap().into()
                ),
                (
                    ToolContext::parse("deno").unwrap(),
                    UnresolvedVersionSpec::parse("7.8.9").unwrap().into()
                ),
            ])
        );
    }

    #[test]
    fn ignores_env_file_when_mode_not_matching() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(
            ".prototools.production",
            r#"
node = "1.2.3"
"#,
        );

        sandbox.create_file(
            ".prototools",
            r#"
node = "7.8.9"
deno = "7.8.9"
"#,
        );

        let manager = ProtoFileManager::load(
            sandbox.path(),
            Some(sandbox.path().parent().unwrap()),
            Some(&"development".to_owned()),
        )
        .unwrap();
        let config = manager.get_local_config(sandbox.path()).unwrap();

        assert_eq!(
            config.versions,
            BTreeMap::from_iter([
                (
                    ToolContext::parse("node").unwrap(),
                    UnresolvedVersionSpec::parse("7.8.9").unwrap().into()
                ),
                (
                    ToolContext::parse("deno").unwrap(),
                    UnresolvedVersionSpec::parse("7.8.9").unwrap().into()
                ),
            ])
        );
    }

    mod lockfile {
        use super::*;

        #[test]
        fn supports_nested_locks() {
            let sandbox = create_empty_sandbox();

            sandbox.create_file(
                "one/.prototools",
                r#"
node = "1.2.3"

[settings]
lockfile = true
"#,
            );

            sandbox.create_file(
                "one/.protolock",
                r#"
[[tools.node]]
spec = "1.2.3"
"#,
            );

            sandbox.create_file(
                ".prototools",
                r#"
node = "7.8.9"
deno = "7.8.9"

[settings]
lockfile = true
"#,
            );

            sandbox.create_file(
                ".protolock",
                r#"
[[tools.node]]
spec = "7.8.9"
"#,
            );

            let manager = ProtoFileManager::load(
                sandbox.path().join("one"),
                Some(sandbox.path().parent().unwrap()),
                None,
            )
            .unwrap();

            // Each config loads its own lockfile
            let nested_lock = manager
                .get_lock(&sandbox.path().join("one/.prototools"))
                .unwrap()
                .unwrap();

            assert_eq!(nested_lock.path, sandbox.path().join("one/.protolock"));
            assert_eq!(
                nested_lock.tools,
                BTreeMap::from_iter([(
                    Id::raw("node"),
                    vec![LockRecord {
                        spec: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                        ..Default::default()
                    }]
                )])
            );

            let root_lock = manager
                .get_lock(&sandbox.path().join(".prototools"))
                .unwrap()
                .unwrap();

            assert_eq!(root_lock.path, sandbox.path().join(".protolock"));
            assert_eq!(
                root_lock.tools,
                BTreeMap::from_iter([(
                    Id::raw("node"),
                    vec![LockRecord {
                        spec: Some(UnresolvedVersionSpec::parse("7.8.9").unwrap()),
                        ..Default::default()
                    }]
                )])
            );

            // And each tool is routed to the config that defines it
            assert_eq!(
                get_locked_config_path(&manager, "node"),
                Some(sandbox.path().join("one/.prototools"))
            );
            assert_eq!(
                get_locked_config_path(&manager, "deno"),
                Some(sandbox.path().join(".prototools"))
            );
        }

        #[test]
        fn doesnt_apply_parent_lock_to_tools_in_nested_configs() {
            let sandbox = create_empty_sandbox();

            sandbox.create_file(
                "one/.prototools",
                r#"
node = "1.2.3"
"#,
            );

            sandbox.create_file(
                ".prototools",
                r#"
node = "7.8.9"
deno = "7.8.9"

[settings]
lockfile = true
"#,
            );

            let manager = ProtoFileManager::load(
                sandbox.path().join("one"),
                Some(sandbox.path().parent().unwrap()),
                None,
            )
            .unwrap();

            // Defined in the nested (unlocked) config
            assert_eq!(get_locked_config_path(&manager, "node"), None);

            // Defined in the locked config
            assert_eq!(
                get_locked_config_path(&manager, "deno"),
                Some(sandbox.path().join(".prototools"))
            );

            // Not defined anywhere, owned by the closest config (unlocked)
            assert_eq!(get_locked_config_path(&manager, "bun"), None);
        }

        #[test]
        fn applies_lock_to_adhoc_tools_from_closest_config() {
            let sandbox = create_empty_sandbox();

            sandbox.create_file(
                ".prototools",
                r#"
node = "7.8.9"

[settings]
lockfile = true
"#,
            );

            // No configs in the nested dir, so the parent owns the scope
            let manager = ProtoFileManager::load(
                sandbox.path().join("one/two"),
                Some(sandbox.path().parent().unwrap()),
                None,
            )
            .unwrap();

            assert_eq!(
                get_locked_config_path(&manager, "node"),
                Some(sandbox.path().join(".prototools"))
            );
            assert_eq!(
                get_locked_config_path(&manager, "bun"),
                Some(sandbox.path().join(".prototools"))
            );
        }

        #[test]
        fn loads_from_a_local_dir() {
            let sandbox = create_empty_sandbox();

            sandbox.create_file(
                "one/.prototools",
                r#"
node = "1.2.3"
"#,
            );

            sandbox.create_file(
                ".prototools",
                r#"
node = "7.8.9"

[settings]
lockfile = true
"#,
            );

            sandbox.create_file(
                ".protolock",
                r#"
[[tools.node]]
spec = "7.8.9"
"#,
            );

            sandbox.create_file(
                ".proto/.prototools",
                r#"
bun = "1.2.3"
"#,
            );

            let manager = ProtoFileManager::load(
                sandbox.path().join("one"),
                Some(sandbox.path().parent().unwrap()),
                None,
            )
            .unwrap();
            let lockfile = manager
                .get_lock(&sandbox.path().join(".prototools"))
                .unwrap()
                .unwrap();

            assert_eq!(
                lockfile.tools,
                BTreeMap::from_iter([(
                    Id::raw("node"),
                    vec![LockRecord {
                        spec: Some(UnresolvedVersionSpec::parse("7.8.9").unwrap()),
                        ..Default::default()
                    }]
                )])
            );

            // The nested config defines node, so the lock doesn't apply to it
            assert_eq!(get_locked_config_path(&manager, "node"), None);
        }

        #[test]
        fn doesnt_load_if_setting_not_enabled() {
            let sandbox = create_empty_sandbox();

            sandbox.create_file(
                ".prototools",
                r#"
node = "1.2.3"
"#,
            );

            sandbox.create_file(
                ".protolock",
                r#"
[[tools.node]]
spec = "1.2.3"
"#,
            );

            let manager = ProtoFileManager::load(
                sandbox.path(),
                Some(sandbox.path().parent().unwrap()),
                None,
            )
            .unwrap();

            assert!(
                manager
                    .get_lock(&sandbox.path().join(".prototools"))
                    .unwrap()
                    .is_none()
            );

            // Now testing false
            sandbox.create_file(
                ".prototools",
                r#"
node = "1.2.3"

[settings]
lockfile = false
"#,
            );

            let manager = ProtoFileManager::load(
                sandbox.path(),
                Some(sandbox.path().parent().unwrap()),
                None,
            )
            .unwrap();

            assert!(
                manager
                    .get_lock(&sandbox.path().join(".prototools"))
                    .unwrap()
                    .is_none()
            );
        }

        #[test]
        fn doesnt_load_if_global_dir() {
            let sandbox = create_empty_sandbox();

            sandbox.create_file(
                ".proto/.prototools",
                r#"
node = "1.2.3"

[settings]
lockfile = true
"#,
            );

            sandbox.create_file(
                ".proto/.protolock",
                r#"
[[tools.node]]
spec = "1.2.3"
"#,
            );

            let manager = ProtoFileManager::load(
                sandbox.path(),
                Some(sandbox.path().parent().unwrap()),
                None,
            )
            .unwrap();

            assert!(
                manager
                    .get_lock(&sandbox.path().join(".proto/.prototools"))
                    .unwrap()
                    .is_none()
            );
        }

        #[test]
        fn doesnt_load_if_user_dir() {
            let sandbox = create_empty_sandbox();

            sandbox.create_file(
                ".home/.prototools",
                r#"
node = "1.2.3"

[settings]
lockfile = true
"#,
            );

            sandbox.create_file(
                ".home/.protolock",
                r#"
[[tools.node]]
spec = "1.2.3"
"#,
            );

            let manager = ProtoFileManager::load(
                sandbox.path(),
                Some(sandbox.path().parent().unwrap()),
                None,
            )
            .unwrap();

            assert!(
                manager
                    .get_lock(&sandbox.path().join(".home/.prototools"))
                    .unwrap()
                    .is_none()
            );
        }

        #[test]
        fn deletes_file_if_not_enabled() {
            let sandbox = create_empty_sandbox();

            sandbox.create_file(
                ".prototools",
                r#"
node = "7.8.9"

[settings]
lockfile = false
"#,
            );

            sandbox.create_file(
                ".protolock",
                r#"
[[tools.node]]
spec = "7.8.9"
"#,
            );

            let manager = ProtoFileManager::load(
                sandbox.path().join("one"),
                Some(sandbox.path().parent().unwrap()),
                None,
            )
            .unwrap();

            assert!(
                manager
                    .get_lock(&sandbox.path().join(".prototools"))
                    .unwrap()
                    .is_none()
            );
            assert!(!sandbox.path().join(".protolock").exists());
        }

        mod env_mode {
            use super::*;

            #[test]
            fn loads_a_lockfile_for_each_config() {
                let sandbox = create_empty_sandbox();

                sandbox.create_file(
                    ".prototools.production",
                    r#"
node = "1.2.3"

[settings]
lockfile = true
"#,
                );

                sandbox.create_file(
                    ".protolock.production",
                    r#"
[[tools.node]]
spec = "1.2.3"
"#,
                );

                sandbox.create_file(
                    ".prototools",
                    r#"
node = "7.8.9"
deno = "7.8.9"

[settings]
lockfile = true
"#,
                );

                sandbox.create_file(
                    ".protolock",
                    r#"
[[tools.node]]
spec = "7.8.9"
"#,
                );

                let manager = ProtoFileManager::load(
                    sandbox.path(),
                    Some(sandbox.path().parent().unwrap()),
                    Some(&"production".to_owned()),
                )
                .unwrap();

                // The env config is locked to its own lockfile
                let env_lock = manager
                    .get_lock(&sandbox.path().join(".prototools.production"))
                    .unwrap()
                    .unwrap();

                assert_eq!(env_lock.path, sandbox.path().join(".protolock.production"));
                assert_eq!(
                    env_lock.tools,
                    BTreeMap::from_iter([(
                        Id::raw("node"),
                        vec![LockRecord {
                            spec: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                            ..Default::default()
                        }]
                    )])
                );

                // And the base config to its own
                let base_lock = manager
                    .get_lock(&sandbox.path().join(".prototools"))
                    .unwrap()
                    .unwrap();

                assert_eq!(base_lock.path, sandbox.path().join(".protolock"));
                assert_eq!(
                    base_lock.tools,
                    BTreeMap::from_iter([(
                        Id::raw("node"),
                        vec![LockRecord {
                            spec: Some(UnresolvedVersionSpec::parse("7.8.9").unwrap()),
                            ..Default::default()
                        }]
                    )])
                );

                // The env config takes precedence for node,
                // while deno is only defined in the base config
                assert_eq!(
                    get_locked_config_path(&manager, "node"),
                    Some(sandbox.path().join(".prototools.production"))
                );
                assert_eq!(
                    get_locked_config_path(&manager, "deno"),
                    Some(sandbox.path().join(".prototools"))
                );

                // Ad-hoc tools are owned by the base config
                assert_eq!(
                    get_locked_config_path(&manager, "bun"),
                    Some(sandbox.path().join(".prototools"))
                );

                assert!(sandbox.path().join(".protolock").exists());
                assert!(sandbox.path().join(".protolock.production").exists());
            }

            #[test]
            fn loads_if_only_an_env_file_exists() {
                let sandbox = create_empty_sandbox();

                sandbox.create_file(
                    ".prototools.production",
                    r#"
node = "7.8.9"

[settings]
lockfile = true
"#,
                );

                sandbox.create_file(
                    ".protolock.production",
                    r#"
[[tools.node]]
spec = "7.8.9"
"#,
                );

                let manager = ProtoFileManager::load(
                    sandbox.path().join("one"),
                    Some(sandbox.path().parent().unwrap()),
                    Some(&"production".to_owned()),
                )
                .unwrap();

                let lockfile = manager
                    .get_lock(&sandbox.path().join(".prototools.production"))
                    .unwrap()
                    .unwrap();

                assert_eq!(
                    lockfile.tools,
                    BTreeMap::from_iter([(
                        Id::raw("node"),
                        vec![LockRecord {
                            spec: Some(UnresolvedVersionSpec::parse("7.8.9").unwrap()),
                            ..Default::default()
                        }]
                    )])
                );

                // The base config doesn't exist, so it has no lockfile
                assert!(
                    manager
                        .get_lock(&sandbox.path().join(".prototools"))
                        .unwrap()
                        .is_none()
                );

                assert_eq!(
                    get_locked_config_path(&manager, "node"),
                    Some(sandbox.path().join(".prototools.production"))
                );

                // Ad-hoc tools fall back to the env config
                // when the base config doesn't exist
                assert_eq!(
                    get_locked_config_path(&manager, "bun"),
                    Some(sandbox.path().join(".prototools.production"))
                );
            }

            #[test]
            fn inherits_setting_from_base_config() {
                let sandbox = create_empty_sandbox();

                sandbox.create_file(
                    ".prototools.production",
                    r#"
node = "1.2.3"
"#,
                );

                sandbox.create_file(
                    ".prototools",
                    r#"
node = "7.8.9"

[settings]
lockfile = true
"#,
                );

                let manager = ProtoFileManager::load(
                    sandbox.path(),
                    Some(sandbox.path().parent().unwrap()),
                    Some(&"production".to_owned()),
                )
                .unwrap();

                // Both configs are locked, even though only the base
                // config enabled the setting
                assert!(
                    manager
                        .get_lock(&sandbox.path().join(".prototools.production"))
                        .unwrap()
                        .is_some()
                );
                assert!(
                    manager
                        .get_lock(&sandbox.path().join(".prototools"))
                        .unwrap()
                        .is_some()
                );

                assert_eq!(
                    get_locked_config_path(&manager, "node"),
                    Some(sandbox.path().join(".prototools.production"))
                );
            }

            #[test]
            fn can_disable_setting_inherited_from_base_config() {
                let sandbox = create_empty_sandbox();

                sandbox.create_file(
                    ".prototools.production",
                    r#"
node = "1.2.3"

[settings]
lockfile = false
"#,
                );

                sandbox.create_file(
                    ".protolock.production",
                    r#"
[[tools.node]]
spec = "1.2.3"
"#,
                );

                sandbox.create_file(
                    ".prototools",
                    r#"
node = "7.8.9"
deno = "7.8.9"

[settings]
lockfile = true
"#,
                );

                sandbox.create_file(
                    ".protolock",
                    r#"
[[tools.node]]
spec = "7.8.9"
"#,
                );

                let manager = ProtoFileManager::load(
                    sandbox.path(),
                    Some(sandbox.path().parent().unwrap()),
                    Some(&"production".to_owned()),
                )
                .unwrap();

                // The env lockfile is disabled and removed
                assert!(
                    manager
                        .get_lock(&sandbox.path().join(".prototools.production"))
                        .unwrap()
                        .is_none()
                );
                assert!(!sandbox.path().join(".protolock.production").exists());

                // But the base lockfile is untouched
                assert!(
                    manager
                        .get_lock(&sandbox.path().join(".prototools"))
                        .unwrap()
                        .is_some()
                );
                assert!(sandbox.path().join(".protolock").exists());

                // The env config defines node but is not locked, and
                // never falls through to the base config
                assert_eq!(get_locked_config_path(&manager, "node"), None);
                assert_eq!(
                    get_locked_config_path(&manager, "deno"),
                    Some(sandbox.path().join(".prototools"))
                );
            }

            #[test]
            fn doesnt_apply_setting_from_env_config_to_base_config() {
                let sandbox = create_empty_sandbox();

                sandbox.create_file(
                    ".prototools.production",
                    r#"
node = "1.2.3"

[settings]
lockfile = true
"#,
                );

                sandbox.create_file(
                    ".prototools",
                    r#"
node = "7.8.9"
deno = "7.8.9"
"#,
                );

                sandbox.create_file(
                    ".protolock",
                    r#"
[[tools.node]]
spec = "7.8.9"
"#,
                );

                let manager = ProtoFileManager::load(
                    sandbox.path(),
                    Some(sandbox.path().parent().unwrap()),
                    Some(&"production".to_owned()),
                )
                .unwrap();

                assert!(
                    manager
                        .get_lock(&sandbox.path().join(".prototools.production"))
                        .unwrap()
                        .is_some()
                );

                // The base config didn't enable the setting,
                // so its lockfile is removed
                assert!(
                    manager
                        .get_lock(&sandbox.path().join(".prototools"))
                        .unwrap()
                        .is_none()
                );
                assert!(!sandbox.path().join(".protolock").exists());

                assert_eq!(
                    get_locked_config_path(&manager, "node"),
                    Some(sandbox.path().join(".prototools.production"))
                );
                assert_eq!(get_locked_config_path(&manager, "deno"), None);
            }

            #[test]
            fn ignores_env_lockfile_when_mode_not_defined() {
                let sandbox = create_empty_sandbox();

                sandbox.create_file(
                    ".prototools.production",
                    r#"
node = "1.2.3"

[settings]
lockfile = true
"#,
                );

                sandbox.create_file(
                    ".protolock.production",
                    r#"
[[tools.node]]
spec = "1.2.3"
"#,
                );

                sandbox.create_file(
                    ".prototools",
                    r#"
node = "7.8.9"

[settings]
lockfile = true
"#,
                );

                let manager = ProtoFileManager::load(
                    sandbox.path(),
                    Some(sandbox.path().parent().unwrap()),
                    None,
                )
                .unwrap();

                assert!(
                    manager
                        .get_lock(&sandbox.path().join(".prototools.production"))
                        .unwrap()
                        .is_none()
                );
                assert!(
                    manager
                        .get_lock(&sandbox.path().join(".prototools"))
                        .unwrap()
                        .is_some()
                );

                // Lockfiles for inactive environments are never touched
                assert!(sandbox.path().join(".protolock.production").exists());

                assert_eq!(
                    get_locked_config_path(&manager, "node"),
                    Some(sandbox.path().join(".prototools"))
                );
            }

            #[test]
            fn ignores_env_lockfile_when_mode_not_matching() {
                let sandbox = create_empty_sandbox();

                sandbox.create_file(
                    ".prototools.production",
                    r#"
node = "1.2.3"

[settings]
lockfile = true
"#,
                );

                sandbox.create_file(
                    ".protolock.production",
                    r#"
[[tools.node]]
spec = "1.2.3"
"#,
                );

                sandbox.create_file(
                    ".prototools",
                    r#"
node = "7.8.9"

[settings]
lockfile = true
"#,
                );

                let manager = ProtoFileManager::load(
                    sandbox.path(),
                    Some(sandbox.path().parent().unwrap()),
                    Some(&"development".to_owned()),
                )
                .unwrap();

                assert!(
                    manager
                        .get_lock(&sandbox.path().join(".prototools.production"))
                        .unwrap()
                        .is_none()
                );
                assert!(
                    manager
                        .get_lock(&sandbox.path().join(".prototools.development"))
                        .unwrap()
                        .is_none()
                );

                // Lockfiles for inactive environments are never touched
                assert!(sandbox.path().join(".protolock.production").exists());

                assert_eq!(
                    get_locked_config_path(&manager, "node"),
                    Some(sandbox.path().join(".prototools"))
                );
            }

            #[test]
            fn deletes_env_lockfile_if_config_doesnt_exist() {
                let sandbox = create_empty_sandbox();

                sandbox.create_file(
                    ".protolock.production",
                    r#"
[[tools.node]]
spec = "1.2.3"
"#,
                );

                sandbox.create_file(
                    ".prototools",
                    r#"
node = "7.8.9"

[settings]
lockfile = true
"#,
                );

                let manager = ProtoFileManager::load(
                    sandbox.path(),
                    Some(sandbox.path().parent().unwrap()),
                    Some(&"production".to_owned()),
                )
                .unwrap();

                assert!(
                    manager
                        .get_lock(&sandbox.path().join(".prototools.production"))
                        .unwrap()
                        .is_none()
                );
                assert!(!sandbox.path().join(".protolock.production").exists());

                // Not defined in the env config, so it falls through
                assert_eq!(
                    get_locked_config_path(&manager, "node"),
                    Some(sandbox.path().join(".prototools"))
                );
                assert_eq!(
                    get_locked_config_path(&manager, "bun"),
                    Some(sandbox.path().join(".prototools"))
                );
            }

            #[test]
            fn supports_nested_locks() {
                let sandbox = create_empty_sandbox();

                sandbox.create_file(
                    "one/.prototools.production",
                    r#"
node = "1.2.3"
"#,
                );

                sandbox.create_file(
                    "one/.prototools",
                    r#"
[settings]
lockfile = true
"#,
                );

                sandbox.create_file(
                    ".prototools",
                    r#"
node = "7.8.9"
deno = "7.8.9"

[settings]
lockfile = true
"#,
                );

                let manager = ProtoFileManager::load(
                    sandbox.path().join("one"),
                    Some(sandbox.path().parent().unwrap()),
                    Some(&"production".to_owned()),
                )
                .unwrap();

                assert_eq!(
                    get_locked_config_path(&manager, "node"),
                    Some(sandbox.path().join("one/.prototools.production"))
                );
                assert_eq!(
                    get_locked_config_path(&manager, "deno"),
                    Some(sandbox.path().join(".prototools"))
                );
                assert_eq!(
                    get_locked_config_path(&manager, "bun"),
                    Some(sandbox.path().join("one/.prototools"))
                );
            }
        }
    }
}
