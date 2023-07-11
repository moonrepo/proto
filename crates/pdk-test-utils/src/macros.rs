#[macro_export]
macro_rules! generate_download_install_tests {
    ($name:expr, $version:expr) => {
        #[tokio::test]
        async fn downloads_verifies_installs_tool() {
            use proto_core::*;

            let sandbox = create_empty_sandbox();
            let plugin = create_plugin($name, sandbox.path());
            let bin_params = plugin.locate_bins(LocateBinsInput {
                env: plugin.tool.get_environment().unwrap(),
                ..LocateBinsInput::default()
            });

            let mut tool = plugin.tool;
            let proto = Proto::from(sandbox.path());

            std::env::set_var("PROTO_ROOT", sandbox.path().to_string_lossy().to_string());

            tool.setup($version).await.unwrap();

            std::env::remove_var("PROTO_ROOT");

            // Check install dir exists
            let base_dir = proto.tools_dir.join($name).join($version);

            assert_eq!(tool.get_install_dir().unwrap(), base_dir);
            assert!(base_dir.exists());

            // Check bin path exists
            assert_eq!(
                tool.get_bin_path().unwrap(),
                &base_dir.join(bin_params.bin_path.unwrap_or($name.into()))
            );

            // Check global bin exists
            assert!(proto
                .bin_dir
                .join(if cfg!(windows) {
                    format!("{}.cmd", $name)
                } else {
                    $name.into()
                })
                .exists());
        }

        #[tokio::test]
        async fn downloads_prebuilt_and_checksum_to_temp() {
            use proto_core::*;

            let sandbox = create_empty_sandbox();
            let plugin = create_plugin($name, sandbox.path());
            let mut tool = plugin.tool;

            tool.version = Some(String::from($version));

            let download_file = tool.get_download_path().unwrap();
            let checksum_file = tool.get_checksum_path().unwrap();

            assert!(!download_file.exists());
            assert!(!checksum_file.exists());

            tool.download(&download_file, None).await.unwrap();

            assert!(download_file.exists());

            if tool.get_checksum_url().unwrap().is_some() {
                tool.download_checksum(&checksum_file, None).await.unwrap();

                assert!(checksum_file.exists());
            }
        }

        #[tokio::test]
        async fn doesnt_download_if_file_exists() {
            use proto_core::*;

            let sandbox = create_empty_sandbox();
            let plugin = create_plugin($name, sandbox.path());
            let mut tool = plugin.tool;

            tool.version = Some(String::from($version));

            let download_file = tool.get_download_path().unwrap();
            let checksum_file = tool.get_checksum_path().unwrap();

            assert!(tool.download(&download_file, None).await.unwrap());
            assert!(!tool.download(&download_file, None).await.unwrap());

            if tool.get_checksum_url().unwrap().is_some() {
                assert!(tool.download_checksum(&checksum_file, None).await.unwrap());
                assert!(!tool.download_checksum(&checksum_file, None).await.unwrap());
            }
        }

        #[tokio::test]
        #[should_panic(expected = "InstallMissingDownload")]
        async fn errors_for_missing_downloads_when_installing() {
            use proto_core::*;

            let sandbox = create_empty_sandbox();
            let plugin = create_plugin($name, sandbox.path());
            let mut tool = plugin.tool;

            tool.version = Some(String::from($version));

            let dir = tool.get_install_dir().unwrap();

            tool.install(&dir, &tool.get_download_path().unwrap())
                .await
                .unwrap();
        }

        #[tokio::test]
        async fn doesnt_install_if_dir_exists() {
            use proto_core::*;

            let sandbox = create_empty_sandbox();
            let plugin = create_plugin($name, sandbox.path());
            let tool = plugin.tool;

            let dir = tool.get_install_dir().unwrap();

            std::fs::create_dir_all(&dir).unwrap();

            assert!(!tool
                .install(&dir, &tool.get_download_path().unwrap())
                .await
                .unwrap());
        }

        #[tokio::test]
        #[should_panic(expected = "VerifyInvalidChecksum")]
        async fn errors_for_checksum_mismatch() {
            use proto_core::*;

            let sandbox = create_empty_sandbox();
            let plugin = create_plugin($name, sandbox.path());
            let mut tool = plugin.tool;

            if tool.get_checksum_url().unwrap().is_none() {
                return;
            }

            tool.version = Some(String::from($version));

            let download_file = tool.get_download_path().unwrap();
            let checksum_file = tool.get_checksum_path().unwrap();

            tool.download(&download_file, None).await.unwrap();
            tool.download_checksum(&checksum_file, None).await.unwrap();

            // Empty the checksum file
            std::fs::write(&checksum_file, "").unwrap();

            tool.verify_checksum(&download_file, &checksum_file)
                .await
                .unwrap();
        }
    };
}
