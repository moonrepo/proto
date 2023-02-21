use assert_fs::prelude::{FileWriteStr, PathChild};
use proto_core::{
    Detector, Downloadable, Executable, Installable, Proto, Resolvable, Tool, Verifiable, Version,
};
use proto_go::GoLanguage;
use std::fs;

fn create_tool() -> (GoLanguage, assert_fs::TempDir) {
    let fixture = assert_fs::TempDir::new().unwrap();
    let tool = GoLanguage::new(&Proto::from(fixture.path()));
    (tool, fixture)
}

#[tokio::test]
async fn downloads_verifies_installs_tool() {
    let fixture = assert_fs::TempDir::new().unwrap();
    let proto = Proto::from(fixture.path());
    let mut tool = GoLanguage::new(&proto);

    std::env::set_var("PROTO_ROOT", fixture.path().to_string_lossy().to_string());

    // Test zero patches because they are weird (go1.20)
    tool.setup("1.20.0").await.unwrap();

    assert!(tool.get_install_dir().unwrap().exists());

    let base_dir = proto.tools_dir.join("go/1.20.0");
    let global_shim = proto.shims_dir.join("go");

    if cfg!(windows) {
        assert_eq!(tool.get_bin_path().unwrap(), &base_dir.join("bin/go.exe"));
        assert!(!global_shim.exists());
    } else {
        assert_eq!(tool.get_bin_path().unwrap(), &base_dir.join("bin/go"));
        assert!(global_shim.exists());
    }
}

mod detector {
    use super::*;

    #[tokio::test]
    async fn doesnt_match_if_no_files() {
        let (tool, fixture) = create_tool();

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn detects_gomod() {
        let (tool, fixture) = create_tool();

        fixture.child("go.mod").write_str("go 1.19").unwrap();

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            Some("1.19".into())
        );
    }

    #[tokio::test]
    async fn detects_gowork() {
        let (tool, fixture) = create_tool();

        fixture.child("go.work").write_str("go 1.19").unwrap();
        fixture.child("go.mod").write_str("go 1.18").unwrap();

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            Some("1.19".into())
        );
    }

    #[tokio::test]
    async fn detects_multiline() {
        let (tool, fixture) = create_tool();

        fixture
            .child("go.mod")
            .write_str("module github.com/moonbase/go_example/server\n\ngo 1.19\n")
            .unwrap();

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            Some("1.19".into())
        );
    }
}

mod downloader {
    use super::*;
    use proto_go::download::get_archive_file;

    #[tokio::test]
    async fn sets_path_to_temp() {
        let (mut tool, fixture) = create_tool();
        tool.version = Some(String::from("1.17.1"));
        assert_eq!(
            tool.get_download_path().unwrap(),
            Proto::from(fixture.path())
                .temp_dir
                .join("go")
                .join(get_archive_file("1.17.1").unwrap())
        );
    }

    #[tokio::test]
    async fn downloads_to_temp() {
        let (mut tool, _fixture) = create_tool();
        tool.version = Some(String::from("1.17.1"));

        let to_file = tool.get_download_path().unwrap();

        assert!(!to_file.exists());

        tool.download(&to_file, None).await.unwrap();

        assert!(to_file.exists());
    }

    #[tokio::test]
    async fn doesnt_download_if_file_exists() {
        let (mut tool, _fixture) = create_tool();
        tool.version = Some(String::from("1.17.1"));

        let to_file = tool.get_download_path().unwrap();

        assert!(tool.download(&to_file, None).await.unwrap());
        assert!(!tool.download(&to_file, None).await.unwrap());
    }

    #[tokio::test]
    async fn downloads_no_patch_versions() {
        let (mut tool, _fixture) = create_tool();
        tool.version = Some(String::from("1.20.0"));

        let to_file = tool.get_download_path().unwrap();
        assert!(!to_file.exists());

        tool.download(&to_file, None).await.unwrap();

        assert!(to_file.exists());
    }
}

mod installer {
    use super::*;

    #[tokio::test]
    async fn sets_dir_to_tools() {
        let (mut tool, fixture) = create_tool();
        tool.version = Some(String::from("1.17.1"));

        assert_eq!(
            tool.get_install_dir().unwrap(),
            Proto::from(fixture.path())
                .tools_dir
                .join("go")
                .join("1.17.1")
        );
    }

    #[tokio::test]
    #[should_panic(expected = "InstallMissingDownload(\"Go\")")]
    async fn errors_for_missing_download() {
        let (mut tool, _fixture) = create_tool();
        tool.version = Some(String::from("1.17.1"));

        let dir = tool.get_install_dir().unwrap();

        tool.install(&dir, &tool.get_download_path().unwrap())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn doesnt_install_if_dir_exists() {
        let (mut tool, _fixture) = create_tool();
        tool.version = Some(String::from("1.17.1"));

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

        assert_ne!(tool.resolve_version("1.19").await.unwrap(), "1.19");
        assert_ne!(tool.resolve_version("1.19").await.unwrap(), "1.19.0");
    }

    #[tokio::test]
    async fn resolve_alias_version() {
        let (mut tool, _fixture) = create_tool();

        assert_eq!(tool.resolve_version("1.11").await.unwrap(), "1.11.13");
    }

    #[tokio::test]
    async fn resolve_specific_version() {
        let (mut tool, _fixture) = create_tool();

        assert_eq!(tool.resolve_version("1.9.2").await.unwrap(), "1.9.2");
    }

    #[tokio::test]
    async fn resolve_rc_version() {
        let (mut tool, _fixture) = create_tool();

        assert_eq!(
            tool.resolve_version("1.9.0-rc2").await.unwrap(),
            "1.9.0-rc2"
        );
    }

    #[tokio::test]
    async fn resolve_latest_version() {
        let (mut tool, _fixture) = create_tool();

        let latest = tool.resolve_version("latest").await.unwrap();
        let latest_version = Version::parse(latest.as_str()).unwrap();
        let current_latest_version = Version::parse("1.19.5").unwrap();

        assert!(latest_version > current_latest_version);
    }
}

mod verifier {
    use super::*;

    #[tokio::test]
    async fn sets_path_to_temp() {
        let (mut tool, fixture) = create_tool();
        tool.version = Some(String::from("1.17.1"));

        assert_eq!(
            tool.get_checksum_path().unwrap(),
            Proto::from(fixture.path())
                .temp_dir
                .join("go")
                .join("1.17.1-SHASUMS256.txt")
        );
    }

    #[tokio::test]
    async fn downloads_to_temp() {
        let (mut tool, _fixture) = create_tool();
        tool.version = Some(String::from("1.17.1"));

        let to_file = tool.get_checksum_path().unwrap();

        assert!(!to_file.exists());

        tool.download_checksum(&to_file, None).await.unwrap();

        assert!(to_file.exists());
    }

    #[tokio::test]
    async fn doesnt_download_if_file_exists() {
        let (mut tool, _fixture) = create_tool();
        tool.version = Some(String::from("1.17.1"));

        let to_file = tool.get_checksum_path().unwrap();

        assert!(tool.download_checksum(&to_file, None).await.unwrap());
        assert!(!tool.download_checksum(&to_file, None).await.unwrap());
    }

    #[tokio::test]
    #[should_panic(expected = "VerifyInvalidChecksum")]
    async fn errors_for_checksum_mismatch() {
        let (mut tool, _fixture) = create_tool();
        tool.version = Some(String::from("1.17.1"));

        let dl_path = tool.get_download_path().unwrap();
        let cs_path = tool.get_checksum_path().unwrap();

        tool.download(&dl_path, None).await.unwrap();
        tool.download_checksum(&cs_path, None).await.unwrap();

        // Empty the checksum file
        fs::write(&cs_path, "").unwrap();

        tool.verify_checksum(&cs_path, &dl_path).await.unwrap();
    }
}
