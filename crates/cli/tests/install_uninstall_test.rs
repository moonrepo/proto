mod utils;

use proto_core::{Id, PinType, ProtoConfig, ToolManifest, UnresolvedVersionSpec, VersionSpec};
use starbase_sandbox::predicates::prelude::*;
use std::collections::HashSet;
use utils::*;

mod install_uninstall {
    use super::*;

    #[test]
    fn installs_without_patch() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("18.12")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        assert!(sandbox.path().join(".proto/tools/node/18.12.1").exists());
    }

    #[test]
    fn installs_without_minor() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("17")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        assert!(sandbox.path().join(".proto/tools/node/17.9.1").exists());
    }

    #[test]
    fn installs_from_alias() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("gallium")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        assert!(sandbox.path().join(".proto/tools/node/16.20.2").exists());
    }

    #[test]
    fn installs_and_uninstalls_tool() {
        let sandbox = create_empty_sandbox();
        let tool_dir = sandbox.path().join(".proto/tools/node/19.0.0");

        assert!(!tool_dir.exists());

        // Install
        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        assert!(tool_dir.exists());

        assert.stderr(predicate::str::contains("Node.js has been installed"));

        // Uninstall
        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("uninstall").arg("node").arg("19.0.0").assert();

        assert!(!tool_dir.exists());

        assert.stderr(predicate::str::contains(
            "Node.js 19.0.0 has been uninstalled!",
        ));
    }

    #[test]
    fn doesnt_install_tool_if_exists() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        assert.stderr(predicate::str::contains(
            "Node.js has already been installed",
        ));
    }

    #[test]
    fn creates_all_shims() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        if cfg!(windows) {
            assert!(sandbox.path().join(".proto/shims/node").exists());
            assert!(sandbox.path().join(".proto/shims/node.cmd").exists());
            assert!(sandbox.path().join(".proto/shims/node.ps1").exists());
        } else {
            assert!(sandbox.path().join(".proto/shims/node").exists());
        }
    }

    #[test]
    fn updates_the_manifest_when_installing() {
        let sandbox = create_empty_sandbox();
        let manifest_file = sandbox.path().join(".proto/tools/node/manifest.json");

        // Install
        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        let manifest = ToolManifest::load(&manifest_file).unwrap();
        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config.versions.get("node").unwrap(),
            &UnresolvedVersionSpec::parse("19.0.0").unwrap()
        );
        assert_eq!(
            manifest.installed_versions,
            HashSet::from_iter([VersionSpec::parse("19.0.0").unwrap()])
        );
        assert!(manifest
            .versions
            .contains_key(&VersionSpec::parse("19.0.0").unwrap()));

        // Uninstall
        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("uninstall")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        let manifest = ToolManifest::load(&manifest_file).unwrap();
        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(config.versions.get("node"), None);
        assert_eq!(manifest.installed_versions, HashSet::default());
        assert!(!manifest
            .versions
            .contains_key(&VersionSpec::parse("19.0.0").unwrap()));
    }

    mod pin {
        use super::*;

        #[test]
        fn can_pin_when_installing() {
            let sandbox = create_empty_sandbox();
            let manifest_file = sandbox.path().join(".proto/tools/node/manifest.json");

            ProtoConfig::update(sandbox.path().join(".proto"), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("18.0.0").unwrap(),
                );
            })
            .unwrap();

            let mut manifest = ToolManifest::load(&manifest_file).unwrap();
            manifest
                .installed_versions
                .insert(VersionSpec::parse("18.0.0").unwrap());
            manifest.save().unwrap();

            let mut cmd = create_proto_command(sandbox.path());
            cmd.arg("install")
                .arg("node")
                .arg("19.0.0")
                .arg("--pin")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert();

            let manifest = ToolManifest::load(&manifest_file).unwrap();
            let config = load_config(sandbox.path().join(".proto"));

            assert_eq!(
                config.versions.get("node").unwrap(),
                &UnresolvedVersionSpec::parse("19.0.0").unwrap()
            );
            assert_eq!(
                manifest.installed_versions,
                HashSet::from_iter([
                    VersionSpec::parse("18.0.0").unwrap(),
                    VersionSpec::parse("19.0.0").unwrap(),
                ])
            );
        }

        #[test]
        fn can_pin_when_already_installed() {
            let sandbox = create_empty_sandbox();

            let mut cmd = create_proto_command(sandbox.path());
            cmd.arg("install")
                .arg("node")
                .arg("19.0.0")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert();

            // Manually change it to something else
            ProtoConfig::update(sandbox.path().join(".proto"), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("18.0.0").unwrap(),
                );
            })
            .unwrap();

            let mut cmd = create_proto_command(sandbox.path());
            cmd.arg("install")
                .arg("node")
                .arg("19.0.0")
                .arg("--pin")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert();

            let config = load_config(sandbox.path().join(".proto"));

            assert_eq!(
                config.versions.get("node").unwrap(),
                &UnresolvedVersionSpec::parse("19.0.0").unwrap()
            );
        }

        #[test]
        fn can_pin_latest_locally_using_setting() {
            let sandbox = create_empty_sandbox();

            // Local
            ProtoConfig::update(sandbox.path(), |config| {
                config.settings.get_or_insert(Default::default()).pin_latest = Some(PinType::Local);

                config.versions.get_or_insert(Default::default()).insert(
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("16.0.0").unwrap(),
                );
            })
            .unwrap();

            // Global
            ProtoConfig::update(sandbox.path().join(".proto"), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("18.0.0").unwrap(),
                );
            })
            .unwrap();

            let mut cmd = create_proto_command(sandbox.path());
            cmd.arg("install")
                .arg("node")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert();

            let global_config = load_config(sandbox.path().join(".proto"));

            assert_eq!(
                global_config.versions.get("node").unwrap(),
                &UnresolvedVersionSpec::parse("18.0.0").unwrap()
            );

            let local_config = load_config(sandbox.path());

            assert_ne!(
                local_config.versions.get("node").unwrap(),
                &UnresolvedVersionSpec::parse("16.0.0").unwrap()
            );
        }

        #[test]
        fn can_pin_latest_globally_using_setting() {
            let sandbox = create_empty_sandbox();

            // Local
            ProtoConfig::update(sandbox.path(), |config| {
                config.settings.get_or_insert(Default::default()).pin_latest =
                    Some(PinType::Global);

                config.versions.get_or_insert(Default::default()).insert(
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("16.0.0").unwrap(),
                );
            })
            .unwrap();

            // Global
            ProtoConfig::update(sandbox.path().join(".proto"), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("18.0.0").unwrap(),
                );
            })
            .unwrap();

            let mut cmd = create_proto_command(sandbox.path());
            cmd.arg("install")
                .arg("node")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert();

            let global_config = load_config(sandbox.path().join(".proto"));

            assert_ne!(
                global_config.versions.get("node").unwrap(),
                &UnresolvedVersionSpec::parse("18.0.0").unwrap()
            );

            let local_config = load_config(sandbox.path());

            assert_eq!(
                local_config.versions.get("node").unwrap(),
                &UnresolvedVersionSpec::parse("16.0.0").unwrap()
            );
        }

        #[test]
        fn doesnt_pin_using_setting_if_not_latest() {
            let sandbox = create_empty_sandbox();

            // Local
            ProtoConfig::update(sandbox.path(), |config| {
                config.settings.get_or_insert(Default::default()).pin_latest = Some(PinType::Local);
            })
            .unwrap();

            let mut cmd = create_proto_command(sandbox.path());
            cmd.arg("install")
                .arg("node")
                .arg("20.0.0")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert();

            let local_config = load_config(sandbox.path());

            assert_eq!(local_config.versions.get("node"), None);
        }
    }

    mod bins {
        use super::*;

        #[cfg(not(windows))]
        #[test]
        fn symlinks_bin_when_pinning() {
            let sandbox = create_empty_sandbox();

            let mut cmd = create_proto_command(sandbox.path());
            cmd.arg("install")
                .arg("node")
                .arg("19.0.0")
                .arg("--pin")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert();

            let link = sandbox.path().join(".proto/bin").join("node");

            assert!(link.exists());

            assert_eq!(
                std::fs::read_link(link).unwrap(),
                sandbox
                    .path()
                    .join(".proto/tools/node/19.0.0")
                    .join("bin/node")
            );
        }

        #[cfg(not(windows))]
        #[test]
        fn symlinks_bin_on_first_install_without_pinning() {
            let sandbox = create_empty_sandbox();

            let mut cmd = create_proto_command(sandbox.path());
            cmd.arg("install")
                .arg("node")
                .arg("19.0.0")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert();

            let link = sandbox.path().join(".proto/bin").join("node");

            assert!(link.exists());
        }

        #[cfg(windows)]
        #[test]
        fn creates_bin_when_pinning() {
            let sandbox = create_empty_sandbox();

            let mut cmd = create_proto_command(sandbox.path());
            cmd.arg("install")
                .arg("node")
                .arg("19.0.0")
                .arg("--pin")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert();

            let link = sandbox.path().join(".proto/bin").join("node.exe");

            assert!(link.exists());
        }

        #[cfg(windows)]
        #[test]
        fn creates_bin_on_first_install_without_pinning() {
            let sandbox = create_empty_sandbox();

            let mut cmd = create_proto_command(sandbox.path());
            cmd.arg("install")
                .arg("node")
                .arg("19.0.0")
                .arg("--")
                .arg("--no-bundled-npm")
                .assert();

            let link = sandbox.path().join(".proto/bin").join("node.exe");

            assert!(link.exists());
        }
    }
}
