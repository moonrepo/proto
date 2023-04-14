use assert_fs::prelude::{FileWriteStr, PathChild};
use proto_core::{
    Detector, Downloadable, Executable, Installable, Proto, Resolvable, Tool, Verifiable,
};
use proto_schema_plugin::InstallSchema;
use proto_schema_plugin::{DetectSchema, Schema, SchemaPlugin};
use rustc_hash::FxHashMap;
use starbase_utils::string_vec;
use std::env::consts;
use std::path::Path;

fn create_plugin(dir: &Path, schema: Schema) -> SchemaPlugin {
    let mut tool = SchemaPlugin::new(Proto::from(dir), schema);
    tool.version = Some("1.0.0".into());
    tool
}

mod schema_plugin {
    use super::*;

    mod detector {
        use super::*;

        #[tokio::test]
        async fn doesnt_match_if_no_files() {
            let fixture = assert_fs::TempDir::new().unwrap();
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
            let fixture = assert_fs::TempDir::new().unwrap();
            let tool = create_plugin(
                fixture.path(),
                Schema {
                    detect: DetectSchema {
                        version_files: Some(string_vec![".version"]),
                    },
                    ..Schema::default()
                },
            );

            fixture.child(".version").write_str("1.2.3").unwrap();

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
                bin: "moon".into(),
                install: InstallSchema {
                    download_url: "https://github.com/moonrepo/moon/releases/download/v{version}/{download_file}".into(),
                    download_file: FxHashMap::from_iter([
                      ("linux".into(), "moon-{arch}-unknown-linux-{libc}".into()),
                      ("macos".into(), "moon-{arch}-apple-darwin".into()),
                      ("windows".into(), "moon-{arch}-pc-windows-msvc.exe".into()),
                    ]),
                    ..InstallSchema::default()
                },
                ..Schema::default()
            }
        }

        #[tokio::test]
        async fn sets_correct_files_urls() {
            let fixture = assert_fs::TempDir::new().unwrap();
            let tool = create_plugin(fixture.path(), create_download_schema());

            if cfg!(target_os = "windows") {
                assert_eq!(
                    tool.get_download_file(),
                    format!("moon-{}-pc-windows-msvc.exe", consts::ARCH)
                );
            } else if cfg!(target_os = "macos") {
                assert_eq!(
                    tool.get_download_file(),
                    format!("moon-{}-apple-darwin", consts::ARCH)
                );
            } else {
                assert_eq!(
                    tool.get_download_file(),
                    format!("moon-{}-unknown-linux-gnu", consts::ARCH)
                );
            }

            assert_eq!(
                tool.get_download_path().unwrap(),
                Proto::from(fixture.path())
                    .temp_dir
                    .join("moon")
                    .join(tool.get_download_file())
            );

            assert_eq!(
                tool.get_download_url().unwrap(),
                format!(
                    "https://github.com/moonrepo/moon/releases/download/v1.0.0/{}",
                    tool.get_download_file()
                )
            );
        }

        #[tokio::test]
        async fn downloads_to_temp() {
            let fixture = assert_fs::TempDir::new().unwrap();
            let tool = create_plugin(fixture.path(), create_download_schema());

            let to_file = tool.get_download_path().unwrap();

            assert!(!to_file.exists());

            tool.download(&to_file, None).await.unwrap();

            assert!(to_file.exists());
        }

        #[tokio::test]
        async fn doesnt_download_if_file_exists() {
            let fixture = assert_fs::TempDir::new().unwrap();
            let tool = create_plugin(fixture.path(), create_download_schema());

            let to_file = tool.get_download_path().unwrap();

            assert!(tool.download(&to_file, None).await.unwrap());
            assert!(!tool.download(&to_file, None).await.unwrap());
        }
    }

    mod verifier {
        use super::*;

        fn create_verify_schema() -> Schema {
            Schema {
                bin: "moon".into(),
                install: InstallSchema {
                    checksum_url: Some("https://github.com/moonrepo/moon/releases/download/v{version}/{checksum_file}".into()),
                    checksum_file: FxHashMap::from_iter([
                      ("linux".into(), "moon-{arch}-unknown-linux-{libc}.sha256".into()),
                      ("macos".into(), "moon-{arch}-apple-darwin.sha256".into()),
                      ("windows".into(), "moon-{arch}-pc-windows-msvc.sha256".into()),
                    ]),
                    ..InstallSchema::default()
                },
                ..Schema::default()
            }
        }

        #[tokio::test]
        async fn sets_correct_files_urls() {
            let fixture = assert_fs::TempDir::new().unwrap();
            let tool = create_plugin(fixture.path(), create_verify_schema());

            if cfg!(target_os = "windows") {
                assert_eq!(
                    tool.get_checksum_file(),
                    format!("moon-{}-pc-windows-msvc.sha256", consts::ARCH)
                );
            } else if cfg!(target_os = "macos") {
                assert_eq!(
                    tool.get_checksum_file(),
                    format!("moon-{}-apple-darwin.sha256", consts::ARCH)
                );
            } else {
                assert_eq!(
                    tool.get_checksum_file(),
                    format!("moon-{}-unknown-linux-gnu.sha256", consts::ARCH)
                );
            }

            assert_eq!(
                tool.get_checksum_path().unwrap(),
                Proto::from(fixture.path())
                    .temp_dir
                    .join("moon")
                    .join(tool.get_checksum_file())
            );

            assert_eq!(
                tool.get_checksum_url().unwrap().unwrap(),
                format!(
                    "https://github.com/moonrepo/moon/releases/download/v1.0.0/{}",
                    tool.get_checksum_file()
                )
            );
        }
    }
}
