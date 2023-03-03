use proto_bun::BunLanguage;
use proto_core::{
    Downloadable, Executable, Installable, Proto, Resolvable, Tool, Verifiable, Version,
};
use std::fs;

fn create_tool() -> (BunLanguage, assert_fs::TempDir) {
    let fixture = assert_fs::TempDir::new().unwrap();
    let mut tool = BunLanguage::new(&Proto::from(fixture.path()));
    tool.version = Some(String::from("0.5.7"));

    (tool, fixture)
}

// Bun doesn't support windows yet!
#[cfg(not(windows))]
mod bun {
    use super::*;

    #[tokio::test]
    async fn downloads_verifies_installs_tool() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let proto = Proto::from(fixture.path());
        let mut tool = BunLanguage::new(&proto);

        std::env::set_var("PROTO_ROOT", fixture.path().to_string_lossy().to_string());

        tool.setup("0.5.7").await.unwrap();

        assert!(tool.get_install_dir().unwrap().exists());

        let base_dir = proto.tools_dir.join("bun/0.5.7");
        let global_shim = proto.bin_dir.join("bun");

        assert_eq!(tool.get_bin_path().unwrap(), &base_dir.join("bun"));
        assert!(global_shim.exists());
    }

    mod downloader {
        use super::*;
        use proto_bun::download::get_archive_file;

        #[tokio::test]
        async fn sets_path_to_temp() {
            let (tool, fixture) = create_tool();

            assert_eq!(
                tool.get_download_path().unwrap(),
                Proto::from(fixture.path())
                    .temp_dir
                    .join("bun")
                    .join(format!("v0.5.7-{}", get_archive_file().unwrap()))
            );
        }

        #[tokio::test]
        async fn downloads_to_temp() {
            let (tool, _fixture) = create_tool();

            let to_file = tool.get_download_path().unwrap();

            assert!(!to_file.exists());

            tool.download(&to_file, None).await.unwrap();

            assert!(to_file.exists());
        }

        #[tokio::test]
        async fn doesnt_download_if_file_exists() {
            let (tool, _fixture) = create_tool();

            let to_file = tool.get_download_path().unwrap();

            assert!(tool.download(&to_file, None).await.unwrap());
            assert!(!tool.download(&to_file, None).await.unwrap());
        }
    }

    mod installer {
        use super::*;

        #[tokio::test]
        async fn sets_dir_to_tools() {
            let (tool, fixture) = create_tool();

            assert_eq!(
                tool.get_install_dir().unwrap(),
                Proto::from(fixture.path())
                    .tools_dir
                    .join("bun")
                    .join("0.5.7")
            );
        }

        #[tokio::test]
        #[should_panic(expected = "InstallMissingDownload(\"Bun\")")]
        async fn errors_for_missing_download() {
            let (mut tool, _fixture) = create_tool();
            tool.version = Some(String::from("0.4.4"));

            let dir = tool.get_install_dir().unwrap();

            tool.install(&dir, &tool.get_download_path().unwrap())
                .await
                .unwrap();
        }

        #[tokio::test]
        async fn doesnt_install_if_dir_exists() {
            let (tool, _fixture) = create_tool();

            let dir = tool.get_install_dir().unwrap();

            fs::create_dir_all(&dir).unwrap();

            assert!(!tool
                .install(&dir, &tool.get_download_path().unwrap())
                .await
                .unwrap());
        }
    }

    mod resolver {
        use super::*;

        #[tokio::test]
        async fn resolve_base_version() {
            let (mut tool, _fixture) = create_tool();
            tool.version = None;

            assert_ne!(tool.resolve_version("0.4").await.unwrap(), "0.4");
        }

        #[tokio::test]
        async fn resolve_alias_version() {
            let (mut tool, _fixture) = create_tool();
            tool.version = None;

            assert_eq!(tool.resolve_version("0.4").await.unwrap(), "0.4.0");
        }

        #[tokio::test]
        async fn resolve_specific_version() {
            let (mut tool, _fixture) = create_tool();
            tool.version = None;

            assert_eq!(tool.resolve_version("0.5.1").await.unwrap(), "0.5.1");
        }

        #[tokio::test]
        async fn resolve_latest_version() {
            let (mut tool, _fixture) = create_tool();
            tool.version = None;

            let latest = tool.resolve_version("latest").await.unwrap();
            let latest_version = Version::parse(latest.as_str()).unwrap();
            let current_latest_version = Version::parse("0.5.7").unwrap();

            assert!(latest_version >= current_latest_version);
        }
    }

    mod verifier {
        use super::*;

        #[tokio::test]
        async fn sets_path_to_temp() {
            let (tool, fixture) = create_tool();

            assert_eq!(
                tool.get_checksum_path().unwrap(),
                Proto::from(fixture.path())
                    .temp_dir
                    .join("bun")
                    .join("v0.5.7-SHASUMS256.txt")
            );
        }

        #[tokio::test]
        async fn downloads_to_temp() {
            let (tool, _fixture) = create_tool();

            let to_file = tool.get_checksum_path().unwrap();

            assert!(!to_file.exists());

            tool.download_checksum(&to_file, None).await.unwrap();

            assert!(to_file.exists());
        }

        #[tokio::test]
        async fn doesnt_download_if_file_exists() {
            let (tool, _fixture) = create_tool();

            let to_file = tool.get_checksum_path().unwrap();

            assert!(tool.download_checksum(&to_file, None).await.unwrap());
            assert!(!tool.download_checksum(&to_file, None).await.unwrap());
        }

        #[tokio::test]
        #[should_panic(expected = "VerifyInvalidChecksum")]
        async fn errors_for_checksum_mismatch() {
            let (tool, _fixture) = create_tool();

            let dl_path = tool.get_download_path().unwrap();
            let cs_path = tool.get_checksum_path().unwrap();

            tool.download(&dl_path, None).await.unwrap();
            tool.download_checksum(&cs_path, None).await.unwrap();

            // Empty the checksum file
            fs::write(&cs_path, "").unwrap();

            tool.verify_checksum(&cs_path, &dl_path).await.unwrap();
        }
    }
}
