use proto_core::{LockRecord, ProtoFileManager, ToolContext};
use starbase_sandbox::create_empty_sandbox;
use std::collections::BTreeMap;
use version_spec::UnresolvedVersionSpec;
use warpgate::{FileLocator, Id, PluginLocator};

mod file_manager {
    use super::*;

    #[test]
    fn merges_traversing_upwards() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(
            "one/two/three/.prototools",
            r#"
node = "1.2.3"

[plugins]
node = "file://./node.toml"
"#,
        );

        sandbox.create_file(
            "one/two/.prototools",
            r#"
[plugins]
bun = "file://../bun.wasm"
"#,
        );

        sandbox.create_file(
            "one/.prototools",
            r#"
bun = "4.5.6"

[plugins]
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
            config.plugins.get("node").unwrap(),
            &PluginLocator::File(Box::new(FileLocator {
                file: "file://./node.toml".into(),
                path: Some(sandbox.path().join("one/two/three/./node.toml"))
            }))
        );

        assert_eq!(
            config.plugins.get("bun").unwrap(),
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
        #[should_panic(expected = "AlreadyLocked")]
        fn errors_for_nested_locks() {
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
                ".prototools",
                r#"
node = "7.8.9"

[settings]
lockfile = true
"#,
            );

            ProtoFileManager::load(
                sandbox.path().join("one"),
                Some(sandbox.path().parent().unwrap()),
                None,
            )
            .unwrap();
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
            let lockfile = manager.get_lock().unwrap();

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
                ".protolock",
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
            let lockfile = manager.get_lock().unwrap();

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

            assert!(manager.get_lock().is_none());

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

            assert!(manager.get_lock().is_none());
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

            assert!(manager.get_lock().is_none());
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

            assert!(manager.get_lock().is_none());
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

            assert!(manager.get_lock().is_none());
            assert!(!sandbox.path().join(".protolock").exists());
        }
    }
}
