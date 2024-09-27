mod utils;

use proto_core::{Id, PinType, ProtoConfig, ToolManifest, UnresolvedVersionSpec, VersionSpec};
use rustc_hash::FxHashSet;
use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod install_uninstall {
    use std::{fs, time::SystemTime};

    use super::*;

    #[test]
    fn installs_without_patch() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("node")
                    .arg("18.12")
                    .arg("--")
                    .arg("--no-bundled-npm");
            })
            .success();

        assert!(sandbox.path().join(".proto/tools/node/18.12.1").exists());
    }

    #[test]
    fn installs_without_minor() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("node")
                    .arg("17")
                    .arg("--")
                    .arg("--no-bundled-npm");
            })
            .success();

        assert!(sandbox.path().join(".proto/tools/node/17.9.1").exists());
    }

    #[test]
    fn installs_from_alias() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("node")
                    .arg("gallium")
                    .arg("--")
                    .arg("--no-bundled-npm");
            })
            .success();

        assert!(sandbox.path().join(".proto/tools/node/16.20.2").exists());
    }

    #[test]
    fn installs_and_uninstalls_tool() {
        let sandbox = create_empty_proto_sandbox();
        let tool_dir = sandbox.path().join(".proto/tools/node/19.0.0");

        assert!(!tool_dir.exists());

        // Install
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("--no-bundled-npm");
            })
            .success();

        assert!(tool_dir.exists());

        assert.stdout(predicate::str::contains(
            "Node.js 19.0.0 has been installed",
        ));

        // Uninstall

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("node").arg("19.0.0");
            })
            .success();

        assert!(!tool_dir.exists());

        assert.stdout(predicate::str::contains(
            "Node.js 19.0.0 has been uninstalled!",
        ));
    }

    #[test]
    fn install_and_reinstall_canary_tool() {
        let sandbox = create_empty_proto_sandbox();
        let tool_dir = sandbox.path().join(".proto/tools/node/canary");
        let tool_bin = if cfg!(windows) {
            sandbox.path().join(".proto/tools/node/canary/node.exe")
        } else {
            sandbox.path().join(".proto/tools/node/canary/bin/node")
        };

        assert!(!tool_dir.exists());
        assert!(!tool_bin.exists());

        // Install
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("node")
                    .arg("canary")
                    .arg("--")
                    .arg("--no-bundled-npm");
            })
            .success();

        assert!(tool_dir.exists());
        assert!(tool_bin.exists());

        assert.stdout(predicate::str::contains(
            "Node.js canary has been installed to",
        ));

        // Support for ctime in tmpfs requires recent linux kernel (>6.11)
        // touch the downloaded bin to modify the mtime instead
        let mtime_original = fs::metadata(tool_bin.clone()).unwrap().modified().unwrap();
        fs::File::options()
            .write(true)
            .open(tool_bin.clone())
            .unwrap()
            .set_modified(SystemTime::now())
            .unwrap();
        let mtime = fs::metadata(tool_bin.clone()).unwrap().modified().unwrap();
        assert_ne!(mtime, mtime_original);

        // Install without --force
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("node")
                    .arg("canary")
                    .arg("--")
                    .arg("--no-bundled-npm");
            })
            .success();

        assert!(tool_dir.exists());
        assert!(tool_bin.exists());

        let mtime_no_reinstall = fs::metadata(tool_bin.clone()).unwrap().modified().unwrap();
        assert_eq!(mtime, mtime_no_reinstall);

        assert.stdout(predicate::str::contains(
            "Node.js canary has already been installed at",
        ));

        // Install with --force
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("node")
                    .arg("--force")
                    .arg("canary")
                    .arg("--")
                    .arg("--no-bundled-npm");
            })
            .success();

        assert!(tool_dir.exists());
        assert!(tool_bin.exists());

        let mtime_reinstall = fs::metadata(tool_bin.clone()).unwrap().modified().unwrap();
        assert_ne!(mtime, mtime_reinstall);

        assert.stdout(predicate::str::contains(
            "Node.js canary has been installed to",
        ));
    }

    #[test]
    fn doesnt_install_tool_if_exists() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("--no-bundled-npm");
            })
            .success();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("--no-bundled-npm");
            })
            .success();

        assert.stdout(predicate::str::contains(
            "Node.js 19.0.0 has already been installed",
        ));
    }

    #[test]
    fn creates_all_shims() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("--no-bundled-npm");
            })
            .success();

        if cfg!(windows) {
            assert!(sandbox.path().join(".proto/shims/node.exe").exists());
        } else {
            assert!(sandbox.path().join(".proto/shims/node").exists());
        }

        // Check that the registry was created also
        assert!(sandbox.path().join(".proto/shims/registry.json").exists());
    }

    #[test]
    fn updates_the_manifest_when_installing() {
        let sandbox = create_empty_proto_sandbox();
        let manifest_file = sandbox.path().join(".proto/tools/node/manifest.json");

        // Install

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("--no-bundled-npm");
            })
            .success();

        let manifest = ToolManifest::load(&manifest_file).unwrap();
        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config.versions.get("node").unwrap(),
            &UnresolvedVersionSpec::parse("19.0.0").unwrap()
        );
        assert_eq!(
            manifest.installed_versions,
            FxHashSet::from_iter([VersionSpec::parse("19.0.0").unwrap()])
        );
        assert!(manifest
            .versions
            .contains_key(&VersionSpec::parse("19.0.0").unwrap()));

        // Uninstall
        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("node").arg("19.0.0");
            })
            .success();

        let manifest = ToolManifest::load(&manifest_file).unwrap();
        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(config.versions.get("node"), None);
        assert_eq!(manifest.installed_versions, FxHashSet::default());
        assert!(!manifest
            .versions
            .contains_key(&VersionSpec::parse("19.0.0").unwrap()));
    }

    mod pin {
        use super::*;

        #[test]
        fn can_pin_when_installing() {
            let sandbox = create_empty_proto_sandbox();
            let manifest_file = sandbox.path().join(".proto/tools/node/manifest.json");

            ProtoConfig::update(sandbox.path(), |config| {
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

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("19.0.0")
                        .arg("--pin")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

            let manifest = ToolManifest::load(&manifest_file).unwrap();
            let config = load_config(sandbox.path());

            assert_eq!(
                config.versions.get("node").unwrap(),
                &UnresolvedVersionSpec::parse("19.0.0").unwrap()
            );
            assert_eq!(
                manifest.installed_versions,
                FxHashSet::from_iter([
                    VersionSpec::parse("18.0.0").unwrap(),
                    VersionSpec::parse("19.0.0").unwrap(),
                ])
            );
        }

        #[test]
        fn can_pin_global_explicitly() {
            let sandbox = create_empty_proto_sandbox();
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

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("19.0.0")
                        .arg("--pin")
                        .arg("global")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

            let manifest = ToolManifest::load(&manifest_file).unwrap();
            let config = load_config(sandbox.path().join(".proto"));

            assert_eq!(
                config.versions.get("node").unwrap(),
                &UnresolvedVersionSpec::parse("19.0.0").unwrap()
            );
            assert_eq!(
                manifest.installed_versions,
                FxHashSet::from_iter([
                    VersionSpec::parse("18.0.0").unwrap(),
                    VersionSpec::parse("19.0.0").unwrap(),
                ])
            );
        }

        #[test]
        fn can_pin_local_explicitly() {
            let sandbox = create_empty_proto_sandbox();
            let manifest_file = sandbox.path().join(".proto/tools/node/manifest.json");

            ProtoConfig::update(sandbox.path(), |config| {
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

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("19.0.0")
                        .arg("--pin")
                        .arg("local")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

            let manifest = ToolManifest::load(&manifest_file).unwrap();
            let config = load_config(sandbox.path());

            assert_eq!(
                config.versions.get("node").unwrap(),
                &UnresolvedVersionSpec::parse("19.0.0").unwrap()
            );
            assert_eq!(
                manifest.installed_versions,
                FxHashSet::from_iter([
                    VersionSpec::parse("18.0.0").unwrap(),
                    VersionSpec::parse("19.0.0").unwrap(),
                ])
            );
        }

        #[test]
        fn can_pin_when_already_installed() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("19.0.0")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

            // Manually change it to something else
            ProtoConfig::update(sandbox.path(), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("18.0.0").unwrap(),
                );
            })
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("19.0.0")
                        .arg("--pin")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

            let config = load_config(sandbox.path());

            assert_eq!(
                config.versions.get("node").unwrap(),
                &UnresolvedVersionSpec::parse("19.0.0").unwrap()
            );
        }

        #[test]
        fn can_pin_latest_locally_using_setting() {
            let sandbox = create_empty_proto_sandbox();

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

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

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
            let sandbox = create_empty_proto_sandbox();

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

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

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
            let sandbox = create_empty_proto_sandbox();

            // Local
            ProtoConfig::update(sandbox.path(), |config| {
                config.settings.get_or_insert(Default::default()).pin_latest = Some(PinType::Local);
            })
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("20.0.0")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

            let local_config = load_config(sandbox.path());

            assert_eq!(local_config.versions.get("node"), None);
        }
    }

    mod bins {
        use super::*;

        #[cfg(not(windows))]
        #[test]
        fn symlinks_bin_when_pinning() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("19.0.0")
                        .arg("--pin")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

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
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("19.0.0")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

            let link = sandbox.path().join(".proto/bin").join("node");

            assert!(link.exists());
        }

        #[cfg(windows)]
        #[test]
        fn creates_bin_when_pinning() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("19.0.0")
                        .arg("--pin")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

            let link = sandbox.path().join(".proto/bin").join("node.exe");

            assert!(link.exists());
        }

        #[cfg(windows)]
        #[test]
        fn creates_bin_on_first_install_without_pinning() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("19.0.0")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

            let link = sandbox.path().join(".proto/bin").join("node.exe");

            assert!(link.exists());
        }
    }
}
