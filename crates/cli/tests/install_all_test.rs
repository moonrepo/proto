mod utils;

use proto_core::{LockfileRecord, ToolLockfile, VersionSpec};
use proto_pdk_api::Checksum;
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeMap;
use utils::*;

mod install_all {
    use super::*;

    #[test]
    fn installs_all_tools() {
        let sandbox = create_empty_proto_sandbox();
        let node_path = sandbox.path().join(".proto/tools/node/19.0.0");
        let npm_path = sandbox.path().join(".proto/tools/npm/9.0.0");
        let deno_path = sandbox.path().join(".proto/tools/deno/1.30.0");

        sandbox.create_file(
            ".prototools",
            r#"node = "19.0.0"
npm = "9.0.0"
deno = "1.30.0"
    "#,
        );

        assert!(!node_path.exists());
        assert!(!npm_path.exists());
        assert!(!deno_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install"); // use
            })
            .success();

        assert!(node_path.exists());
        assert!(npm_path.exists());
        assert!(deno_path.exists());
    }

    #[test]
    fn installs_tool_via_detection() {
        let sandbox = create_empty_proto_sandbox();
        let node_path = sandbox.path().join(".proto/tools/node/19.0.0");

        sandbox.create_file(".nvmrc", "19.0.0");

        assert!(!node_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("use"); // install
            })
            .success();

        assert!(node_path.exists());
    }

    #[test]
    fn doesnt_install_global_tools() {
        let sandbox = create_empty_proto_sandbox();
        let node_path = sandbox.path().join(".proto/tools/node/19.0.0");
        let deno_path = sandbox.path().join(".proto/tools/deno/1.30.0");

        sandbox.create_file(".prototools", r#"node = "19.0.0""#);
        sandbox.create_file(".proto/.prototools", r#"deno = "1.30.0""#);

        assert!(!node_path.exists());
        assert!(!deno_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("use");
            })
            .success();

        assert!(node_path.exists());
        assert!(!deno_path.exists());
    }

    #[test]
    fn installs_global_tools_when_included() {
        let sandbox = create_empty_proto_sandbox();
        let node_path = sandbox.path().join(".proto/tools/node/19.0.0");
        let deno_path = sandbox.path().join(".proto/tools/deno/1.30.0");

        sandbox.create_file(".prototools", r#"node = "19.0.0""#);
        sandbox.create_file(".proto/.prototools", r#"deno = "1.30.0""#);

        assert!(!node_path.exists());
        assert!(!deno_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("--config-mode")
                    .arg("upwards-global");
            })
            .success();

        assert!(node_path.exists());
        assert!(deno_path.exists());
    }

    mod reqs {
        use super::*;

        #[test]
        fn errors_if_reqs_not_met() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", r#"npm = "9.0.0""#);

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install");
                })
                .failure();

            assert.stderr(predicate::str::contains(
                "npm requires node to function correctly",
            ));
        }

        #[test]
        fn passes_if_reqs_met() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"node = "19.0.0"
npm = "10.0.0"
        "#,
            );

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install");
                })
                .success();

            assert.stdout(
                predicate::str::contains("Waiting on requirements: node")
                    .and(predicate::str::contains("npm 10.0.0 installed")),
            );
        }
    }

    mod lockfile {
        use super::*;

        #[test]
        fn creates_all_lockfiles() {
            let sandbox = create_empty_proto_sandbox();
            let node_path = sandbox.path().join(".proto/tools/node/19.0.0");
            let npm_path = sandbox.path().join(".proto/tools/npm/9.0.0");
            let deno_path = sandbox.path().join(".proto/tools/deno/1.30.0");

            sandbox.create_file(
                ".prototools",
                r#"node = "19.0.0"
npm = "9.0.0"
deno = "1.30.0"
    "#,
            );

            assert!(!node_path.exists());
            assert!(!npm_path.exists());
            assert!(!deno_path.exists());

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install"); // use
                })
                .success();

            assert!(node_path.exists());
            assert!(npm_path.exists());
            assert!(deno_path.exists());

            #[cfg(target_os = "linux")]
            {
                assert_eq!(
                    ToolLockfile::load(node_path.parent().unwrap().join("lockfile.json")).unwrap().versions,
                    BTreeMap::from_iter([(
                        VersionSpec::parse("19.0.0").unwrap(),
                        LockfileRecord {
                            checksum: Some(Checksum::sha256(
                                "a16fa0fd4ba7dff0f9476778dbabe535250c99a121db4c65c2a68a2506097698"
                                    .into()
                            )),
                            source: Some("https://nodejs.org/download/release/v19.0.0/node-v19.0.0-linux-x64.tar.xz".into()),
                            ..Default::default()
                        }
                    )])
                );

                assert_eq!(
                    ToolLockfile::load(npm_path.parent().unwrap().join("lockfile.json"))
                        .unwrap()
                        .versions,
                    BTreeMap::from_iter([(
                        VersionSpec::parse("9.0.0").unwrap(),
                        LockfileRecord {
                            checksum: Some(Checksum::sha256(
                                "84e7b6c2b573a549782056f4348c76969a90cd861441fa25469545d3600e2ee3"
                                    .into()
                            )),
                            source: Some("https://registry.npmjs.org/npm/-/npm-9.0.0.tgz".into()),
                            ..Default::default()
                        }
                    )])
                );

                assert_eq!(
                    ToolLockfile::load(deno_path.parent().unwrap().join("lockfile.json"))
                        .unwrap()
                        .versions,
                    BTreeMap::from_iter([(
                        VersionSpec::parse("1.30.0").unwrap(),
                        LockfileRecord {
                            checksum: Some(Checksum::sha256(
                                "77ebb253b3bc8ba5ca62b44b60e8b8555c1b3d0011fbcebd1d52291652f834a8"
                                    .into()
                            )),
                            source: Some(
                                "https://dl.deno.land/release/v1.30.0/deno-x86_64-unknown-linux-gnu.zip"
                                    .into()
                            ),
                            ..Default::default()
                        }
                    )])
                );
            }

            #[cfg(target_os = "macos")]
            {
                assert_eq!(
                    ToolLockfile::load(node_path.parent().unwrap().join("lockfile.json")).unwrap().versions,
                    BTreeMap::from_iter([(
                        VersionSpec::parse("19.0.0").unwrap(),
                        LockfileRecord {
                            checksum: Some(Checksum::sha256(
                                "76c550a8f2aa9611ce9148d6d3a5af900c2cbbc4b35ba68d545f63239c2d24e9"
                                    .into()
                            )),
                            source: Some("https://nodejs.org/download/release/v19.0.0/node-v19.0.0-darwin-arm64.tar.xz".into()),
                            ..Default::default()
                        }
                    )])
                );

                assert_eq!(
                    ToolLockfile::load(npm_path.parent().unwrap().join("lockfile.json"))
                        .unwrap()
                        .versions,
                    BTreeMap::from_iter([(
                        VersionSpec::parse("9.0.0").unwrap(),
                        LockfileRecord {
                            checksum: Some(Checksum::sha256(
                                "84e7b6c2b573a549782056f4348c76969a90cd861441fa25469545d3600e2ee3"
                                    .into()
                            )),
                            source: Some("https://registry.npmjs.org/npm/-/npm-9.0.0.tgz".into()),
                            ..Default::default()
                        }
                    )])
                );

                assert_eq!(
                    ToolLockfile::load(deno_path.parent().unwrap().join("lockfile.json"))
                        .unwrap()
                        .versions,
                    BTreeMap::from_iter([(
                        VersionSpec::parse("1.30.0").unwrap(),
                        LockfileRecord {
                            checksum: Some(Checksum::sha256(
                                "80c6a6f9e4dbda8cd024dd6ac39a64306eded98d532efa8bf12ddc9c12626a1d"
                                    .into()
                            )),
                            source: Some(
                                "https://dl.deno.land/release/v1.30.0/deno-aarch64-apple-darwin.zip"
                                    .into()
                            ),
                            ..Default::default()
                        }
                    )])
                );
            }

            #[cfg(target_os = "windows")]
            {
                assert_eq!(
                    ToolLockfile::load(node_path.parent().unwrap().join("lockfile.json")).unwrap().versions,
                    BTreeMap::from_iter([(
                        VersionSpec::parse("19.0.0").unwrap(),
                        LockfileRecord {
                            checksum: Some(Checksum::sha256(
                                "94fdfb96a041b1a9cafd1ee1bb42ab57a5b73f6a3606cd222ae96c5768bdb31d"
                                    .into()
                            )),
                            source: Some("https://nodejs.org/download/release/v19.0.0/node-v19.0.0-win-x64.zip".into()),
                            ..Default::default()
                        }
                    )])
                );

                assert_eq!(
                    ToolLockfile::load(npm_path.parent().unwrap().join("lockfile.json"))
                        .unwrap()
                        .versions,
                    BTreeMap::from_iter([(
                        VersionSpec::parse("9.0.0").unwrap(),
                        LockfileRecord {
                            checksum: Some(Checksum::sha256(
                                "84e7b6c2b573a549782056f4348c76969a90cd861441fa25469545d3600e2ee3"
                                    .into()
                            )),
                            source: Some("https://registry.npmjs.org/npm/-/npm-9.0.0.tgz".into()),
                            ..Default::default()
                        }
                    )])
                );

                assert_eq!(
                    ToolLockfile::load(deno_path.parent().unwrap().join("lockfile.json"))
                        .unwrap()
                        .versions,
                    BTreeMap::from_iter([(
                        VersionSpec::parse("1.30.0").unwrap(),
                        LockfileRecord {
                            checksum: Some(Checksum::sha256(
                                "3644c734d4a21e9db8e3992d081ca0e742e986674a6be0eff113ffc5fa5416eb"
                                    .into()
                            )),
                            source: Some(
                                "https://dl.deno.land/release/v1.30.0/deno-x86_64-pc-windows-msvc.zip"
                                    .into()
                            ),
                            ..Default::default()
                        }
                    )])
                );
            }
        }
    }
}
