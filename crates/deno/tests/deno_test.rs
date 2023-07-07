use proto_core::{
    Detector, Downloadable, Executable, Installable, Proto, Resolvable, Tool, Version,
};
use proto_deno::DenoLanguage;
use starbase_sandbox::{create_empty_sandbox, Sandbox};
use std::env;
use std::fs;

fn create_tool() -> (DenoLanguage, Sandbox) {
    let fixture = create_empty_sandbox();
    let tool = DenoLanguage::new(Proto::from(fixture.path()));
    (tool, fixture)
}

mod deno {
    use super::*;

    #[tokio::test]
    async fn downloads_verifies_installs_tool() {
        let fixture = create_empty_sandbox();
        let proto = Proto::from(fixture.path());
        let mut tool = DenoLanguage::new(&proto);

        env::set_var("PROTO_ROOT", fixture.path().to_string_lossy().to_string());

        tool.setup("1.17.3").await.unwrap();

        env::remove_var("PROTO_ROOT");

        assert!(tool.get_install_dir().unwrap().exists());

        let base_dir = proto.tools_dir.join("deno/1.17.3");

        if cfg!(windows) {
            assert_eq!(tool.get_bin_path().unwrap(), &base_dir.join("deno.exe"));
            assert!(proto.bin_dir.join("deno.cmd").exists());
        } else {
            assert_eq!(tool.get_bin_path().unwrap(), &base_dir.join("deno"));
            assert!(proto.bin_dir.join("deno").exists());
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
        async fn detects_dvmrc() {
            let (tool, fixture) = create_tool();

            fixture.create_file(".dvmrc", "1.30.1");

            assert_eq!(
                tool.detect_version_from(fixture.path()).await.unwrap(),
                Some("1.30.1".into())
            );
        }
    }

    mod downloader {
        use super::*;
        use proto_deno::download::get_archive_file;

        #[tokio::test]
        async fn sets_path_to_temp() {
            let (mut tool, fixture) = create_tool();
            tool.version = Some(String::from("1.28.3"));

            assert_eq!(
                tool.get_download_path().unwrap(),
                Proto::from(fixture.path())
                    .temp_dir
                    .join("deno")
                    .join(format!("v1.28.3-{}", get_archive_file().unwrap()))
            );
        }

        #[tokio::test]
        async fn downloads_to_temp() {
            let (mut tool, _fixture) = create_tool();
            tool.version = Some(String::from("1.28.3"));

            let to_file = tool.get_download_path().unwrap();

            assert!(!to_file.exists());

            tool.download(&to_file, None).await.unwrap();

            assert!(to_file.exists());
        }

        #[tokio::test]
        async fn doesnt_download_if_file_exists() {
            let (mut tool, _fixture) = create_tool();
            tool.version = Some(String::from("1.28.3"));

            let to_file = tool.get_download_path().unwrap();

            assert!(tool.download(&to_file, None).await.unwrap());
            assert!(!tool.download(&to_file, None).await.unwrap());
        }
    }

    mod installer {
        use super::*;

        #[tokio::test]
        async fn sets_dir_to_tools() {
            let (mut tool, fixture) = create_tool();
            tool.version = Some(String::from("1.28.3"));

            assert_eq!(
                tool.get_install_dir().unwrap(),
                Proto::from(fixture.path())
                    .tools_dir
                    .join("deno")
                    .join("1.28.3")
            );
        }

        #[tokio::test]
        #[should_panic(expected = "InstallMissingDownload(\"Deno\")")]
        async fn errors_for_missing_download() {
            let (mut tool, _fixture) = create_tool();
            tool.version = Some(String::from("1.28.3"));

            let dir = tool.get_install_dir().unwrap();

            tool.install(&dir, &tool.get_download_path().unwrap())
                .await
                .unwrap();
        }

        #[tokio::test]
        async fn doesnt_install_if_dir_exists() {
            let (mut tool, _fixture) = create_tool();
            tool.version = Some(String::from("1.28.3"));

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

            assert_eq!(tool.resolve_version("1.11").await.unwrap(), "1.11.5");
        }

        #[tokio::test]
        async fn resolve_specific_version() {
            let (mut tool, _fixture) = create_tool();

            assert_eq!(tool.resolve_version("1.9.2").await.unwrap(), "1.9.2");
        }

        #[tokio::test]
        async fn resolve_latest_version() {
            let (mut tool, _fixture) = create_tool();

            let latest = tool.resolve_version("latest").await.unwrap();
            let latest_version = Version::parse(latest.as_str()).unwrap();
            let current_latest_version = Version::parse("1.19.5").unwrap();

            assert!(latest_version > current_latest_version);
        }

        #[tokio::test]
        async fn resolve_custom_alias() {
            let (mut tool, fixture) = create_tool();

            fixture.create_file(
                "tools/deno/manifest.json",
                r#"{"aliases":{"example":"1.30.0"}}"#,
            );

            assert_eq!(tool.resolve_version("example").await.unwrap(), "1.30.0");
        }
    }
}
