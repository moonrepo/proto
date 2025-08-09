mod utils;

use proto_core::{
    LockRecord, PinLocation, ProtoConfig, ToolContext, ToolManifest, UnresolvedVersionSpec,
    VersionSpec,
};
use proto_pdk_api::Checksum;
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeSet;
use std::fs;
use std::time::SystemTime;
use utils::*;

mod install_one {
    use super::*;

    #[test]
    fn installs_without_patch() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("2.5");
            })
            .success();

        assert!(
            sandbox
                .path()
                .join(".proto/tools/protostar/2.5.15")
                .exists()
        );
    }

    #[test]
    fn installs_without_minor() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("2");
            })
            .success();

        assert!(
            sandbox
                .path()
                .join(".proto/tools/protostar/2.10.15")
                .exists()
        );
    }

    #[test]
    fn installs_from_alias() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("stable");
            })
            .success();

        assert!(sandbox.path().join(".proto/tools/protostar/5.0.0").exists());
    }

    #[test]
    fn installs_via_detection() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".protostarrc", "1.2.3");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar");
            })
            .success();

        assert!(sandbox.path().join(".proto/tools/protostar/1.2.3").exists());
    }

    #[test]
    fn installs_via_prototools() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".prototools", "protostar = \"1.2.3\"");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar");
            })
            .success();

        assert!(sandbox.path().join(".proto/tools/protostar/1.2.3").exists());
    }

    #[test]
    fn installs_latest_if_no_version() {
        let sandbox = create_empty_proto_sandbox();

        assert!(!sandbox.path().join(".proto/tools/protostar").exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar");
            })
            .success();

        assert!(
            sandbox
                .path()
                .join(".proto/tools/protostar/5.10.15")
                .exists()
        );
    }

    #[test]
    fn installs_and_uninstalls_proto() {
        let sandbox = create_empty_proto_sandbox();
        let tool_dir = sandbox.path().join(".proto/tools/proto/0.45.0");

        assert!(!tool_dir.exists());

        // Install
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("proto").arg("0.45.0");
            })
            .success();

        assert!(tool_dir.exists());

        assert.stdout(predicate::str::contains("proto 0.45.0 has been installed"));

        // Uninstall
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("proto").arg("0.45.0").arg("--yes");
            })
            .success();

        assert!(!tool_dir.exists());

        assert.stdout(predicate::str::contains(
            "proto 0.45.0 has been uninstalled!",
        ));
    }

    #[test]
    fn installs_and_uninstalls_tool() {
        let sandbox = create_empty_proto_sandbox();
        let tool_dir = sandbox.path().join(".proto/tools/protostar/5.0.0");

        assert!(!tool_dir.exists());

        // Install
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("5.0.0");
            })
            .success();

        assert!(tool_dir.exists());

        assert.stdout(predicate::str::contains(
            "protostar 5.0.0 has been installed",
        ));

        // Uninstall
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall")
                    .arg("protostar")
                    .arg("5.0.0")
                    .arg("--yes");
            })
            .success();

        assert!(!tool_dir.exists());

        assert.stdout(predicate::str::contains(
            "protostar 5.0.0 has been uninstalled!",
        ));
    }

    #[test]
    fn installs_and_reinstalls_canary_tool() {
        let sandbox = create_empty_proto_sandbox();
        let tool_dir = sandbox.path().join(".proto/tools/protostar/canary");
        let tool_bin = if cfg!(windows) {
            sandbox
                .path()
                .join(".proto/tools/protostar/canary/protostar.exe")
        } else {
            sandbox
                .path()
                .join(".proto/tools/protostar/canary/protostar")
        };

        assert!(!tool_dir.exists());
        assert!(!tool_bin.exists());

        // Install
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("canary");
            })
            .success();

        assert!(tool_dir.exists());
        assert!(tool_bin.exists());

        assert.stdout(predicate::str::contains(
            "protostar canary has been installed to",
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
                cmd.arg("install").arg("protostar").arg("canary");
            })
            .success();

        assert!(tool_dir.exists());
        assert!(tool_bin.exists());

        let mtime_no_reinstall = fs::metadata(tool_bin.clone()).unwrap().modified().unwrap();
        assert_eq!(mtime, mtime_no_reinstall);

        assert.stdout(predicate::str::contains(
            "protostar canary has already been installed at",
        ));

        // Install with --force
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("protostar")
                    .arg("canary")
                    .arg("--force");
            })
            .success();

        assert!(tool_dir.exists());
        assert!(tool_bin.exists());

        let mtime_reinstall = fs::metadata(tool_bin.clone()).unwrap().modified().unwrap();
        assert_ne!(mtime, mtime_reinstall);

        assert.stdout(predicate::str::contains(
            "protostar canary has been installed to",
        ));
    }

    #[test]
    fn subsequent_req_install_doesnt_resolve_from_manifest() {
        let sandbox = create_empty_proto_sandbox();

        // Install a non-latest version
        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("2.5.10");
            })
            .success();

        assert!(
            sandbox
                .path()
                .join(".proto/tools/protostar/2.5.10")
                .exists()
        );

        // Install again with a requirement that should resolve
        // to the latest version
        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("~2.5");
            })
            .success();

        assert!(
            sandbox
                .path()
                .join(".proto/tools/protostar/2.5.15")
                .exists()
        );
    }

    #[test]
    fn doesnt_install_tool_if_exists() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        assert.stdout(predicate::str::contains(
            "protostar 1.0.0 has already been installed",
        ));
    }

    #[test]
    fn creates_all_shims() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        if cfg!(windows) {
            assert!(sandbox.path().join(".proto/shims/protostar.exe").exists());
        } else {
            assert!(sandbox.path().join(".proto/shims/protostar").exists());
        }

        // Check that the registry was created also
        assert!(sandbox.path().join(".proto/shims/registry.json").exists());
    }

    #[test]
    fn updates_the_manifest_when_installing() {
        let sandbox = create_empty_proto_sandbox();
        let manifest_file = sandbox.path().join(".proto/tools/protostar/manifest.json");

        // Install
        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        let manifest = ToolManifest::load(&manifest_file).unwrap();
        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config
                .versions
                .get(&ToolContext::parse("protostar").unwrap())
                .unwrap(),
            &UnresolvedVersionSpec::parse("1.0.0").unwrap()
        );
        assert_eq!(
            manifest.installed_versions,
            BTreeSet::from_iter([VersionSpec::parse("1.0.0").unwrap()])
        );
        assert!(
            manifest
                .versions
                .contains_key(&VersionSpec::parse("1.0.0").unwrap())
        );

        // Uninstall
        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall")
                    .arg("protostar")
                    .arg("1.0.0")
                    .arg("--yes");
            })
            .success();

        let manifest = ToolManifest::load(&manifest_file).unwrap();
        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config
                .versions
                .get(&ToolContext::parse("protostar").unwrap()),
            None
        );
        assert_eq!(manifest.installed_versions, BTreeSet::default());
        assert!(
            !manifest
                .versions
                .contains_key(&VersionSpec::parse("1.0.0").unwrap())
        );
    }

    mod pin {
        use super::*;

        #[test]
        fn can_pin_when_installing() {
            let sandbox = create_empty_proto_sandbox();
            let manifest_file = sandbox.path().join(".proto/tools/protostar/manifest.json");

            ProtoConfig::update(sandbox.path(), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    ToolContext::parse("protostar").unwrap(),
                    UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                );
            })
            .unwrap();

            let mut manifest = ToolManifest::load(&manifest_file).unwrap();
            manifest
                .installed_versions
                .insert(VersionSpec::parse("1.0.0").unwrap());
            manifest.save().unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("2.0.0")
                        .arg("--pin");
                })
                .success();

            let manifest = ToolManifest::load(&manifest_file).unwrap();
            let config = load_config(sandbox.path());

            assert_eq!(
                config
                    .versions
                    .get(&ToolContext::parse("protostar").unwrap())
                    .unwrap(),
                &UnresolvedVersionSpec::parse("2.0.0").unwrap()
            );
            assert_eq!(
                manifest.installed_versions,
                BTreeSet::from_iter([
                    VersionSpec::parse("1.0.0").unwrap(),
                    VersionSpec::parse("2.0.0").unwrap(),
                ])
            );
        }

        #[test]
        fn can_pin_global_explicitly() {
            let sandbox = create_empty_proto_sandbox();
            let manifest_file = sandbox.path().join(".proto/tools/protostar/manifest.json");

            ProtoConfig::update(sandbox.path().join(".proto"), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    ToolContext::parse("protostar").unwrap(),
                    UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                );
            })
            .unwrap();

            let mut manifest = ToolManifest::load(&manifest_file).unwrap();
            manifest
                .installed_versions
                .insert(VersionSpec::parse("1.0.0").unwrap());
            manifest.save().unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("2.0.0")
                        .arg("--pin")
                        .arg("global");
                })
                .success();

            let manifest = ToolManifest::load(&manifest_file).unwrap();
            let config = load_config(sandbox.path().join(".proto"));

            assert_eq!(
                config
                    .versions
                    .get(&ToolContext::parse("protostar").unwrap())
                    .unwrap(),
                &UnresolvedVersionSpec::parse("2.0.0").unwrap()
            );
            assert_eq!(
                manifest.installed_versions,
                BTreeSet::from_iter([
                    VersionSpec::parse("1.0.0").unwrap(),
                    VersionSpec::parse("2.0.0").unwrap(),
                ])
            );
        }

        #[test]
        fn can_pin_local_explicitly() {
            let sandbox = create_empty_proto_sandbox();
            let manifest_file = sandbox.path().join(".proto/tools/protostar/manifest.json");

            ProtoConfig::update(sandbox.path(), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    ToolContext::parse("protostar").unwrap(),
                    UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                );
            })
            .unwrap();

            let mut manifest = ToolManifest::load(&manifest_file).unwrap();
            manifest
                .installed_versions
                .insert(VersionSpec::parse("1.0.0").unwrap());
            manifest.save().unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("2.0.0")
                        .arg("--pin")
                        .arg("local");
                })
                .success();

            let manifest = ToolManifest::load(&manifest_file).unwrap();
            let config = load_config(sandbox.path());

            assert_eq!(
                config
                    .versions
                    .get(&ToolContext::parse("protostar").unwrap())
                    .unwrap(),
                &UnresolvedVersionSpec::parse("2.0.0").unwrap()
            );
            assert_eq!(
                manifest.installed_versions,
                BTreeSet::from_iter([
                    VersionSpec::parse("1.0.0").unwrap(),
                    VersionSpec::parse("2.0.0").unwrap(),
                ])
            );
        }

        #[test]
        fn can_pin_when_already_installed() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("1.0.0");
                })
                .success();

            // Manually change it to something else
            ProtoConfig::update(sandbox.path(), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    ToolContext::parse("protostar").unwrap(),
                    UnresolvedVersionSpec::parse("5.0.0").unwrap().into(),
                );
            })
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("1.0.0")
                        .arg("--pin");
                })
                .success();

            let config = load_config(sandbox.path());

            assert_eq!(
                config
                    .versions
                    .get(&ToolContext::parse("protostar").unwrap())
                    .unwrap(),
                &UnresolvedVersionSpec::parse("1.0.0").unwrap()
            );
        }

        #[test]
        fn can_pin_latest_locally_using_setting() {
            let sandbox = create_empty_proto_sandbox();

            // Local
            ProtoConfig::update(sandbox.path(), |config| {
                config.settings.get_or_insert(Default::default()).pin_latest =
                    Some(PinLocation::Local);

                config.versions.get_or_insert(Default::default()).insert(
                    ToolContext::parse("protostar").unwrap(),
                    UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                );
            })
            .unwrap();

            // Global
            ProtoConfig::update(sandbox.path().join(".proto"), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    ToolContext::parse("protostar").unwrap(),
                    UnresolvedVersionSpec::parse("2.0.0").unwrap().into(),
                );
            })
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("latest");
                })
                .success();

            let global_config = load_config(sandbox.path().join(".proto"));

            assert_eq!(
                global_config
                    .versions
                    .get(&ToolContext::parse("protostar").unwrap())
                    .unwrap(),
                &UnresolvedVersionSpec::parse("2.0.0").unwrap()
            );

            let local_config = load_config(sandbox.path());

            assert_ne!(
                local_config
                    .versions
                    .get(&ToolContext::parse("protostar").unwrap())
                    .unwrap(),
                &UnresolvedVersionSpec::parse("1.0.0").unwrap()
            );
        }

        #[test]
        fn can_pin_latest_globally_using_setting() {
            let sandbox = create_empty_proto_sandbox();

            // Local
            ProtoConfig::update(sandbox.path(), |config| {
                config.settings.get_or_insert(Default::default()).pin_latest =
                    Some(PinLocation::Global);

                config.versions.get_or_insert(Default::default()).insert(
                    ToolContext::parse("protostar").unwrap(),
                    UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                );
            })
            .unwrap();

            // Global
            ProtoConfig::update(sandbox.path().join(".proto"), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    ToolContext::parse("protostar").unwrap(),
                    UnresolvedVersionSpec::parse("2.0.0").unwrap().into(),
                );
            })
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("latest");
                })
                .success();

            let global_config = load_config(sandbox.path().join(".proto"));

            assert_ne!(
                global_config
                    .versions
                    .get(&ToolContext::parse("protostar").unwrap())
                    .unwrap(),
                &UnresolvedVersionSpec::parse("2.0.0").unwrap()
            );

            let local_config = load_config(sandbox.path());

            assert_eq!(
                local_config
                    .versions
                    .get(&ToolContext::parse("protostar").unwrap())
                    .unwrap(),
                &UnresolvedVersionSpec::parse("1.0.0").unwrap()
            );
        }

        #[test]
        fn doesnt_pin_using_setting_if_not_latest() {
            let sandbox = create_empty_proto_sandbox();

            // Local
            ProtoConfig::update(sandbox.path(), |config| {
                config.settings.get_or_insert(Default::default()).pin_latest =
                    Some(PinLocation::Local);
            })
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("1.0.0");
                })
                .success();

            let local_config = load_config(sandbox.path());

            assert_eq!(
                local_config
                    .versions
                    .get(&ToolContext::parse("protostar").unwrap()),
                None
            );
        }
    }

    mod bins {
        use super::*;

        #[cfg(not(windows))]
        #[test]
        fn symlinks_bins() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("1.0.0");
                })
                .success();

            let link1 = sandbox.path().join(".proto/bin/protostar");
            let link2 = sandbox.path().join(".proto/bin/protostar-1");
            let link3 = sandbox.path().join(".proto/bin/protostar-1.0");
            let src = sandbox
                .path()
                .join(".proto/tools/protostar/1.0.0/protostar");

            assert!(link1.exists());
            assert!(link2.exists());
            assert!(link3.exists());

            assert_eq!(std::fs::read_link(link1).unwrap(), src);
            assert_eq!(std::fs::read_link(link2).unwrap(), src);
            assert_eq!(std::fs::read_link(link3).unwrap(), src);
        }

        #[cfg(windows)]
        #[test]
        fn creates_bins() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("1.0.0");
                })
                .success();

            let link1 = sandbox.path().join(".proto/bin/protostar.exe");
            let link2 = sandbox.path().join(".proto/bin/protostar-1.exe");
            let link3 = sandbox.path().join(".proto/bin/protostar-1.0.exe");

            assert!(link1.exists());
            assert!(link2.exists());
            assert!(link3.exists());
        }
    }

    mod requirements {
        use super::*;

        #[test]
        fn errors_if_reqs_not_met() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("moonbase").arg("1.0.0");
                })
                .failure();

            assert.stderr(predicate::str::contains(
                "moonbase requires moonstone to function correctly",
            ));
        }

        #[test]
        fn passes_if_reqs_met() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", r#"moonstone = "2""#);

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("moonbase").arg("1.0.0");
                })
                .success();

            assert.stdout(predicate::str::contains(
                "moonbase 1.0.0 has been installed",
            ));
        }
    }

    mod manifest_lockfile {
        use super::*;

        #[test]
        fn adds_and_removes_lockfile_entries() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("1.0.0");
                })
                .success();

            let mut manifest =
                ToolManifest::load_from(sandbox.path().join(".proto/tools/protostar")).unwrap();
            let lock = manifest
                .versions
                .remove(&VersionSpec::parse("1.0.0").unwrap())
                .unwrap()
                .lock
                .unwrap();

            assert_eq!(
                lock,
                LockRecord {
                    // spec: Some(UnresolvedVersionSpec::parse("1.0.0").unwrap()),
                    // version: Some(VersionSpec::parse("1.0.0").unwrap()),
                    checksum: Some(Checksum::sha256(
                        "92521fc3cbd964bdc9f584a991b89fddaa5754ed1cc96d6d42445338669c1305".into()
                    )),
                    ..Default::default()
                }
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("uninstall")
                        .arg("protostar")
                        .arg("1.0.0")
                        .arg("--yes");
                })
                .success();

            let manifest =
                ToolManifest::load_from(sandbox.path().join(".proto/tools/protostar")).unwrap();

            assert!(manifest.versions.is_empty());
        }

        #[test]
        fn errors_if_checksum_mismatch() {
            let sandbox = create_empty_proto_sandbox();
            let manifest_path = sandbox.path().join(".proto/tools/protostar/manifest.json");

            fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();

            fs::write(
                manifest_path,
                r#"{
    "versions": {
        "1.0.0": {
            "lock": {
                "checksum": "sha256:12345somefakehash67890"
            }
        }
    }
}"#,
            )
            .unwrap();

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("1.0.0");
                })
                .failure();

            assert.stderr(predicate::str::contains("Checksum mismatch"));
        }
    }
}
