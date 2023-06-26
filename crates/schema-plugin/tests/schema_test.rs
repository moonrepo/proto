use proto_core::{
    get_home_dir, Detector, Downloadable, Executable, Installable, Proto, Resolvable, Shimable,
    Verifiable,
};
use proto_schema_plugin::{
    DetectSchema, InstallSchema, PlatformMapper, ResolveSchema, Schema, SchemaPlugin,
};
use rustc_hash::FxHashMap;
use starbase_sandbox::create_empty_sandbox;
use starbase_utils::string_vec;
use std::env::{self, consts};
use std::fs;
use std::path::Path;

fn create_plugin(dir: &Path, mut schema: Schema) -> SchemaPlugin {
    schema.name = "moon-test".into();

    let mut tool = SchemaPlugin::new(Proto::from(dir), "moon-test".into(), schema);
    tool.version = Some("1.0.0".into());
    tool
}

mod schema_plugin {
    use super::*;

    mod detector {
        use super::*;

        #[tokio::test]
        async fn doesnt_match_if_no_files() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(
                fixture.path(),
                Schema {
                    detect: DetectSchema {
                        version_files: Some(string_vec![".version"]),
                    },
                    ..Schema::default()
                },
            );

            assert_eq!(
                tool.detect_version_from(fixture.path()).await.unwrap(),
                None
            );
        }

        #[tokio::test]
        async fn detects_nvm() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(
                fixture.path(),
                Schema {
                    detect: DetectSchema {
                        version_files: Some(string_vec![".version"]),
                    },
                    ..Schema::default()
                },
            );

            fixture.create_file(".version", "1.2.3");

            assert_eq!(
                tool.detect_version_from(fixture.path()).await.unwrap(),
                Some("1.2.3".into())
            );
        }
    }

    mod downloader {
        use super::*;

        fn create_download_schema() -> Schema {
            Schema {
                platform: FxHashMap::from_iter([
                    (
                        "linux".into(),
                        PlatformMapper {
                            download_file: "moon-{arch}-unknown-linux-{libc}".into(),
                                ..PlatformMapper::default()
                        }
                    ),
                    (
                        "macos".into(),
                        PlatformMapper {
                            download_file: "moon-{arch}-apple-darwin".into(),
                            ..PlatformMapper::default()
                        }
                    ),
                    (
                        "windows".into(),
                        PlatformMapper {
                            download_file:"moon-{arch}-pc-windows-msvc.exe".into(),
                            ..PlatformMapper::default()
                        }
                    ),
                ]),
                install: InstallSchema {
                    download_url: "https://github.com/moonrepo/moon/releases/download/v{version}/{download_file}".into(),
                    ..InstallSchema::default()
                },
                ..Schema::default()
            }
        }

        #[tokio::test]
        async fn sets_correct_files_urls() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path(), create_download_schema());

            if cfg!(target_os = "windows") {
                assert_eq!(
                    tool.get_download_file().unwrap(),
                    format!("moon-{}-pc-windows-msvc.exe", consts::ARCH)
                );
            } else if cfg!(target_os = "macos") {
                assert_eq!(
                    tool.get_download_file().unwrap(),
                    format!("moon-{}-apple-darwin", consts::ARCH)
                );
            } else {
                assert_eq!(
                    tool.get_download_file().unwrap(),
                    format!("moon-{}-unknown-linux-gnu", consts::ARCH)
                );
            }

            assert_eq!(
                tool.get_download_path().unwrap(),
                Proto::from(fixture.path())
                    .temp_dir
                    .join("moon-test")
                    .join("1.0.0")
                    .join(tool.get_download_file().unwrap())
            );

            assert_eq!(
                tool.get_download_url().unwrap(),
                format!(
                    "https://github.com/moonrepo/moon/releases/download/v1.0.0/{}",
                    tool.get_download_file().unwrap()
                )
            );
        }

        #[tokio::test]
        async fn downloads_to_temp() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path(), create_download_schema());

            let to_file = tool.get_download_path().unwrap();

            assert!(!to_file.exists());

            tool.download(&to_file, None).await.unwrap();

            assert!(to_file.exists());
        }

        #[tokio::test]
        async fn doesnt_download_if_file_exists() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path(), create_download_schema());

            let to_file = tool.get_download_path().unwrap();

            assert!(tool.download(&to_file, None).await.unwrap());
            assert!(!tool.download(&to_file, None).await.unwrap());
        }
    }

    mod executor {
        use super::*;

        #[tokio::test]
        async fn uses_bin_in_cwd_by_default() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin(fixture.path(), Schema::default());

            let bin_path = tool.get_install_dir().unwrap().join(if cfg!(windows) {
                "moon-test.exe"
            } else {
                "moon-test"
            });

            fs::create_dir_all(bin_path.parent().unwrap()).unwrap();
            fs::write(&bin_path, "").unwrap();

            tool.find_bin_path().await.unwrap();

            assert_eq!(tool.get_bin_path().unwrap(), bin_path);
        }

        #[tokio::test]
        async fn can_customize_based_on_os() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin(
                fixture.path(),
                Schema {
                    platform: FxHashMap::from_iter([
                        (
                            "linux".into(),
                            PlatformMapper {
                                bin_path: Some("lin/moon".into()),
                                ..PlatformMapper::default()
                            },
                        ),
                        (
                            "macos".into(),
                            PlatformMapper {
                                bin_path: Some("mac/moon".into()),
                                ..PlatformMapper::default()
                            },
                        ),
                        (
                            "windows".into(),
                            PlatformMapper {
                                bin_path: Some("win/moon.exe".into()),
                                ..PlatformMapper::default()
                            },
                        ),
                    ]),
                    ..Schema::default()
                },
            );

            let bin_name = if cfg!(target_os = "windows") {
                "win/moon.exe"
            } else if cfg!(target_os = "macos") {
                "mac/moon"
            } else {
                "lin/moon"
            };

            let bin_path = tool.get_install_dir().unwrap().join(bin_name);

            fs::create_dir_all(bin_path.parent().unwrap()).unwrap();
            fs::write(&bin_path, "").unwrap();

            tool.find_bin_path().await.unwrap();

            assert_eq!(tool.get_bin_path().unwrap(), bin_path);
        }

        mod globals {
            use super::*;

            #[tokio::test]
            async fn defaults_to_some_home_dir() {
                let fixture = create_empty_sandbox();
                let tool = create_plugin(fixture.path(), Schema::default());

                assert_eq!(
                    tool.get_globals_bin_dir().unwrap(),
                    get_home_dir().unwrap().join(".moon-test/bin")
                );
            }

            #[tokio::test]
            async fn expands_home_dir() {
                let fixture = create_empty_sandbox();
                let tool = create_plugin(
                    fixture.path(),
                    Schema {
                        install: InstallSchema {
                            globals_dir: string_vec!["~/.moon-test/bin"],
                            ..InstallSchema::default()
                        },
                        ..Schema::default()
                    },
                );
                let bin_dir = get_home_dir().unwrap().join(".moon-test/bin");

                fs::create_dir_all(&bin_dir).unwrap();

                assert_eq!(tool.get_globals_bin_dir().unwrap(), bin_dir);
            }

            #[tokio::test]
            async fn expands_home_env_var() {
                let fixture = create_empty_sandbox();
                let tool = create_plugin(
                    fixture.path(),
                    Schema {
                        install: InstallSchema {
                            globals_dir: string_vec!["$HOME/.moon-test/bin"],
                            ..InstallSchema::default()
                        },
                        ..Schema::default()
                    },
                );
                let bin_dir = get_home_dir().unwrap().join(".moon-test/bin");

                fs::create_dir_all(&bin_dir).unwrap();

                assert_eq!(tool.get_globals_bin_dir().unwrap(), bin_dir);
            }

            #[tokio::test]
            async fn supports_env_vars() {
                let fixture = create_empty_sandbox();
                let tool = create_plugin(
                    fixture.path(),
                    Schema {
                        install: InstallSchema {
                            globals_dir: string_vec!["$PROTO_TEST_DIR/bin"],
                            ..InstallSchema::default()
                        },
                        ..Schema::default()
                    },
                );
                let bin_dir = fixture.path().join("bin");

                fs::create_dir_all(&bin_dir).unwrap();

                env::set_var("PROTO_TEST_DIR", fixture.path());
                assert_eq!(tool.get_globals_bin_dir().unwrap(), bin_dir);
                env::remove_var("PROTO_TEST_DIR");
            }
        }
    }

    mod installer {
        use super::*;

        #[tokio::test]
        async fn can_customize_prefix_based_on_os() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(
                fixture.path(),
                Schema {
                    platform: FxHashMap::from_iter([
                        (
                            "linux".into(),
                            PlatformMapper {
                                archive_prefix: Some("linux-{arch}".into()),
                                ..PlatformMapper::default()
                            },
                        ),
                        (
                            "macos".into(),
                            PlatformMapper {
                                archive_prefix: Some("macos-{arch}".into()),
                                ..PlatformMapper::default()
                            },
                        ),
                        (
                            "windows".into(),
                            PlatformMapper {
                                archive_prefix: Some("windows-{arch}".into()),
                                ..PlatformMapper::default()
                            },
                        ),
                    ]),
                    ..Schema::default()
                },
            );

            let prefix = format!("{}-{}", consts::OS, tool.schema.get_arch());

            assert_eq!(tool.get_archive_prefix().unwrap(), Some(prefix));
        }
    }

    mod resolver {
        use super::*;

        #[tokio::test]
        async fn resolves_git_tags() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(
                fixture.path(),
                Schema {
                    resolve: ResolveSchema {
                        git_url: Some("https://github.com/moonrepo/moon".into()),
                        ..ResolveSchema::default()
                    },
                    ..Schema::default()
                },
            );

            let manifest = tool.load_version_manifest().await.unwrap();

            assert!(!manifest.versions.is_empty());
            assert!(manifest.versions.contains_key("1.0.0"));
            assert!(manifest.aliases.contains_key("latest"));
        }

        #[tokio::test]
        async fn resolves_git_tags_with_custom_pattern() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(
                fixture.path(),
                Schema {
                    resolve: ResolveSchema {
                        git_url: Some("https://github.com/moonrepo/moon".into()),
                        git_tag_pattern: r"^@moonrepo/cli@((\d+)\.(\d+)\.(\d+))".into(),
                        ..ResolveSchema::default()
                    },
                    ..Schema::default()
                },
            );

            let manifest = tool.load_version_manifest().await.unwrap();

            assert!(!manifest.versions.is_empty());
            assert!(manifest.versions.contains_key("1.0.0"));
            assert!(manifest.aliases.contains_key("latest"));
        }

        #[tokio::test]
        async fn resolves_endpoint() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(
                fixture.path(),
                Schema {
                    resolve: ResolveSchema {
                        manifest_url: Some("https://nodejs.org/dist/index.json".into()),
                        ..ResolveSchema::default()
                    },
                    ..Schema::default()
                },
            );

            let manifest = tool.load_version_manifest().await.unwrap();

            assert!(!manifest.versions.is_empty());
            assert!(manifest.versions.contains_key("19.0.0"));
            assert!(manifest.aliases.contains_key("latest"));
        }

        #[tokio::test]
        async fn resolves_endpoint_with_custom_key() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(
                fixture.path(),
                Schema {
                    resolve: ResolveSchema {
                        manifest_url: Some("https://nodejs.org/dist/index.json".into()),
                        manifest_version_key: "npm".into(),
                        ..ResolveSchema::default()
                    },
                    ..Schema::default()
                },
            );

            let manifest = tool.load_version_manifest().await.unwrap();

            assert!(!manifest.versions.is_empty());
            // npm hasn't hit v19 yet!
            assert!(!manifest.versions.contains_key("19.0.0"));
            assert!(manifest.aliases.contains_key("latest"));
        }
    }

    mod shimmer {
        use super::*;

        #[tokio::test]
        async fn creates_global_shim() {
            let fixture = create_empty_sandbox();
            let proto = Proto::from(fixture.path());
            let mut tool = create_plugin(fixture.path(), Schema::default());

            tool.bin_path = Some(proto.bin_dir.join("moon-test"));
            tool.schema.shim.global = true;

            env::set_var("PROTO_ROOT", fixture.path());
            tool.create_shims(false).await.unwrap();
            env::remove_var("PROTO_ROOT");

            if cfg!(windows) {
                assert!(proto.bin_dir.join("moon-test.cmd").exists());
            } else {
                assert!(proto.bin_dir.join("moon-test").exists());
            }
        }

        #[tokio::test]
        async fn creates_local_shim() {
            let fixture = create_empty_sandbox();
            let proto = Proto::from(fixture.path());
            let mut tool = create_plugin(fixture.path(), Schema::default());

            tool.bin_path = Some(proto.bin_dir.join("moon-test"));
            tool.schema.shim.local = true;
            tool.create_shims(false).await.unwrap();

            if cfg!(windows) {
                assert!(tool
                    .get_install_dir()
                    .unwrap()
                    .join("shims\\moon-test.ps1")
                    .exists());
            } else {
                assert!(tool
                    .get_install_dir()
                    .unwrap()
                    .join("shims/moon-test")
                    .exists());
            }
        }
    }

    mod verifier {
        use super::*;

        fn create_verify_schema() -> Schema {
            Schema {
                platform: FxHashMap::from_iter([
                    ("linux".into(), PlatformMapper {
                        checksum_file: Some("moon-{arch}-unknown-linux-{libc}.sha256".into()),
                        ..PlatformMapper::default()
                    }),
                    ("macos".into(), PlatformMapper {
                        checksum_file: Some("moon-{arch}-apple-darwin.sha256".into()),
                        ..PlatformMapper::default()
                    }),
                    ("windows".into(), PlatformMapper {
                        checksum_file:Some("moon-{arch}-pc-windows-msvc.sha256".into()),
                        ..PlatformMapper::default()
                    }),
                ]),
                install: InstallSchema {
                    checksum_url: Some("https://github.com/moonrepo/moon/releases/download/v{version}/{checksum_file}".into()),
                    ..InstallSchema::default()
                },
                ..Schema::default()
            }
        }

        #[tokio::test]
        async fn sets_correct_files_urls() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path(), create_verify_schema());

            if cfg!(target_os = "windows") {
                assert_eq!(
                    tool.get_checksum_file().unwrap(),
                    format!("moon-{}-pc-windows-msvc.sha256", consts::ARCH)
                );
            } else if cfg!(target_os = "macos") {
                assert_eq!(
                    tool.get_checksum_file().unwrap(),
                    format!("moon-{}-apple-darwin.sha256", consts::ARCH)
                );
            } else {
                assert_eq!(
                    tool.get_checksum_file().unwrap(),
                    format!("moon-{}-unknown-linux-gnu.sha256", consts::ARCH)
                );
            }

            assert_eq!(
                tool.get_checksum_path().unwrap(),
                Proto::from(fixture.path())
                    .temp_dir
                    .join("moon-test")
                    .join("1.0.0")
                    .join(tool.get_checksum_file().unwrap())
            );

            assert_eq!(
                tool.get_checksum_url().unwrap().unwrap(),
                format!(
                    "https://github.com/moonrepo/moon/releases/download/v1.0.0/{}",
                    tool.get_checksum_file().unwrap()
                )
            );
        }
    }
}
