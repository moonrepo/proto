mod utils;

use proto_core::{
    Id, LockfileRecord, PinLocation, ProtoConfig, ToolManifest, UnresolvedVersionSpec, VersionSpec,
};
use proto_pdk_api::Checksum;
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeSet;
use std::fs;
use std::time::SystemTime;
use utils::*;

mod install_uninstall {
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
    fn installs_via_detection() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".node-version", "17");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node");
            })
            .success();

        assert!(sandbox.path().join(".proto/tools/node/17.9.1").exists());
    }

    #[test]
    fn installs_via_prototools() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".prototools", "node = \"17\"");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node");
            })
            .success();

        assert!(sandbox.path().join(".proto/tools/node/17.9.1").exists());
    }

    #[test]
    fn installs_latest_if_no_version() {
        let sandbox = create_empty_proto_sandbox();

        assert!(!sandbox.path().join(".proto/tools/node").exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node");
            })
            .success();

        assert!(sandbox.path().join(".proto/tools/node").exists());
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
                cmd.arg("uninstall").arg("node").arg("19.0.0").arg("--yes");
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
            BTreeSet::from_iter([VersionSpec::parse("19.0.0").unwrap()])
        );
        assert!(
            manifest
                .versions
                .contains_key(&VersionSpec::parse("19.0.0").unwrap())
        );

        // Uninstall
        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("node").arg("19.0.0").arg("--yes");
            })
            .success();

        let manifest = ToolManifest::load(&manifest_file).unwrap();
        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(config.versions.get("node"), None);
        assert_eq!(manifest.installed_versions, BTreeSet::default());
        assert!(
            !manifest
                .versions
                .contains_key(&VersionSpec::parse("19.0.0").unwrap())
        );
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
                    UnresolvedVersionSpec::parse("18.0.0").unwrap().into(),
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
                BTreeSet::from_iter([
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
                    UnresolvedVersionSpec::parse("18.0.0").unwrap().into(),
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
                BTreeSet::from_iter([
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
                    UnresolvedVersionSpec::parse("18.0.0").unwrap().into(),
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
                BTreeSet::from_iter([
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
                    UnresolvedVersionSpec::parse("18.0.0").unwrap().into(),
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
                config.settings.get_or_insert(Default::default()).pin_latest =
                    Some(PinLocation::Local);

                config.versions.get_or_insert(Default::default()).insert(
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("16.0.0").unwrap().into(),
                );
            })
            .unwrap();

            // Global
            ProtoConfig::update(sandbox.path().join(".proto"), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("18.0.0").unwrap().into(),
                );
            })
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("latest")
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
                    Some(PinLocation::Global);

                config.versions.get_or_insert(Default::default()).insert(
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("16.0.0").unwrap().into(),
                );
            })
            .unwrap();

            // Global
            ProtoConfig::update(sandbox.path().join(".proto"), |config| {
                config.versions.get_or_insert(Default::default()).insert(
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("18.0.0").unwrap().into(),
                );
            })
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("latest")
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
                config.settings.get_or_insert(Default::default()).pin_latest =
                    Some(PinLocation::Local);
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
        fn symlinks_bins() {
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

            let link1 = sandbox.path().join(".proto/bin/node");
            let link2 = sandbox.path().join(".proto/bin/node-19");
            let link3 = sandbox.path().join(".proto/bin/node-19.0");
            let src = sandbox.path().join(".proto/tools/node/19.0.0/bin/node");

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
                    cmd.arg("install")
                        .arg("node")
                        .arg("19.0.0")
                        .arg("--pin")
                        .arg("--")
                        .arg("--no-bundled-npm");
                })
                .success();

            let link1 = sandbox.path().join(".proto/bin/node.exe");
            let link2 = sandbox.path().join(".proto/bin/node-19.exe");
            let link3 = sandbox.path().join(".proto/bin/node-19.0.exe");

            assert!(link1.exists());
            assert!(link2.exists());
            assert!(link3.exists());
        }
    }

    mod reqs {
        use super::*;

        #[test]
        fn errors_if_reqs_not_met() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("npm").arg("10.0.0");
                })
                .failure();

            assert.stderr(predicate::str::contains(
                "npm requires node to function correctly",
            ));
        }

        #[test]
        fn passes_if_reqs_met() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", r#"node = "20""#);

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("npm").arg("10.0.0");
                })
                .success();

            assert.stdout(predicate::str::contains("npm 10.0.0 has been installed"));
        }
    }

    #[cfg(not(windows))]
    mod backend {
        use super::*;

        #[test]
        fn installs_and_uninstalls_asdf_tool() {
            let sandbox = create_empty_proto_sandbox();
            let tool_dir = sandbox.path().join(".proto/tools/zig/0.13.0");

            assert!(!tool_dir.exists());

            // Install
            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("zig").arg("asdf:0.13.0");
                })
                .success();

            assert!(tool_dir.exists());

            assert.stdout(predicate::str::contains(
                "asdf:zig 0.13.0 has been installed",
            ));

            // Uninstall
            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("uninstall")
                        .arg("zig")
                        .arg("asdf:0.13.0")
                        .arg("--yes");
                })
                .success();

            assert!(!tool_dir.exists());

            assert.stdout(predicate::str::contains(
                "asdf:zig 0.13.0 has been uninstalled!",
            ));
        }

        #[test]
        fn installs_and_pins_backend() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("zig")
                        .arg("asdf:0.13.0")
                        .arg("--pin")
                        .arg("local");
                })
                .success();

            let config = load_config(sandbox.path());

            assert_eq!(
                config.versions.get("zig").unwrap(),
                &ToolSpec::new_backend(
                    UnresolvedVersionSpec::parse("0.13.0").unwrap(),
                    Some(Backend::Asdf)
                )
            );
        }

        #[test]
        fn installs_with_shortname() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
[tools.newrelic]
asdf-shortname = "newrelic-cli"
"#,
            );

            let tool_dir = sandbox.path().join(".proto/tools/newrelic/0.97.0");

            assert!(!tool_dir.exists());

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("newrelic").arg("asdf:0.97.0");
                })
                .success();

            assert!(tool_dir.exists());

            assert.stdout(predicate::str::contains(
                "asdf:newrelic 0.97.0 has been installed",
            ));
        }

        #[test]
        fn installs_with_custom_repo() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"
[tools.newrelic]
asdf-repository = "https://github.com/NeoHsu/asdf-newrelic-cli"
"#,
            );

            let tool_dir = sandbox.path().join(".proto/tools/newrelic/0.97.0");

            assert!(!tool_dir.exists());

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("newrelic").arg("asdf:0.97.0");
                })
                .success();

            assert!(tool_dir.exists());

            assert.stdout(predicate::str::contains(
                "asdf:newrelic 0.97.0 has been installed",
            ));
        }
    }

    mod lockfile {
        use super::*;

        #[test]
        fn adds_and_removes_lockfile_entries() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("node").arg("18.12.0");
                })
                .success();

            let mut manifest =
                ToolManifest::load_from(sandbox.path().join(".proto/tools/node")).unwrap();
            let lock = manifest
                .versions
                .remove(&VersionSpec::parse("18.12.0").unwrap())
                .unwrap()
                .lock
                .unwrap();

            #[cfg(target_os = "linux")]
            assert_eq!(
                lock,
                LockRecord {
                    checksum: Some(Checksum::sha256(
                        "9429e26d9a35cb079897f0a22622fe89ff597976259a8fcb38b7d08b154789dc"
                            .into()
                    )),
                    source: Some("https://nodejs.org/download/release/v18.12.0/node-v18.12.0-linux-x64.tar.xz".into()),
                    ..Default::default()
                }
            );

            #[cfg(target_os = "macos")]
            assert_eq!(
                lock,
                LockRecord {
                    checksum: Some(Checksum::sha256(
                        "e37d6b4fbb4ca4ef3af0a095ff9089d7a5c3c80d4bc36d916987406f06573464"
                            .into()
                    )),
                    source: Some("https://nodejs.org/download/release/v18.12.0/node-v18.12.0-darwin-arm64.tar.xz".into()),
                    ..Default::default()
                }
            );

            #[cfg(target_os = "windows")]
            assert_eq!(
                lock,
                LockRecord {
                    checksum: Some(Checksum::sha256(
                        "56a3a49e0e4701f169bb742ea98f5006800229e2e3bf7e10493642f392416ac8".into()
                    )),
                    source: Some(
                        "https://nodejs.org/download/release/v18.12.0/node-v18.12.0-win-x64.zip"
                            .into()
                    ),
                    ..Default::default()
                }
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("uninstall").arg("node").arg("18.12.0").arg("--yes");
                })
                .success();

            let manifest =
                ToolManifest::load_from(sandbox.path().join(".proto/tools/node")).unwrap();

            assert!(manifest.versions.is_empty());
        }

        #[test]
        fn errors_if_checksum_mismatch() {
            let sandbox = create_empty_proto_sandbox();
            let manifest_path = sandbox.path().join(".proto/tools/node/manifest.json");

            fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();

            fs::write(
                manifest_path,
                r#"{
    "versions": {
        "18.12.0": {
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
                    cmd.arg("install").arg("node").arg("18.12.0");
                })
                .failure();

            assert.stderr(predicate::str::contains("Checksum mismatch"));
        }
    }
}
