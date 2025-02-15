#[macro_export]
macro_rules! generate_build_install_tests {
    ($id:literal, $version:literal) => {
        generate_build_install_tests!($id, $version, None);
    };
    ($id:literal, $version:literal, $schema:expr) => {
        #[tokio::test(flavor = "multi_thread")]
        async fn builds_installs_tool_from_source() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                sandbox.create_schema_plugin($id, schema).await
            } else {
                sandbox.create_plugin($id).await
            };
            let spec = UnresolvedVersionSpec::parse($version).unwrap();

            let result = plugin
                .tool
                .setup(
                    &spec,
                    proto_pdk_test_utils::flow::install::InstallOptions {
                        console: Some(proto_pdk_test_utils::ProtoConsole::new_testing()),
                        strategy: proto_pdk_test_utils::InstallStrategy::BuildFromSource,
                        skip_prompts: true,
                        skip_ui: true,
                        ..Default::default()
                    },
                )
                .await;

            // Print the log so we can debug
            if result.is_err() {
                println!(
                    "{}",
                    std::fs::read_to_string(
                        sandbox.path().join(format!("proto-{}-build.log", $id))
                    )
                    .unwrap()
                );
            }

            result.unwrap();

            // Check install dir exists
            let base_dir = sandbox.proto_dir.join("tools").join($id).join($version);
            let tool_dir = plugin.tool.get_product_dir();

            assert_eq!(tool_dir, base_dir);
            assert!(base_dir.exists());

            // Check bin path exists (would panic)
            plugin.tool.locate_exe_file().await.unwrap();

            // Check things exist
            for bin in plugin.tool.resolve_bin_locations(true).await.unwrap() {
                assert!(bin.path.exists());
            }

            for shim in plugin.tool.resolve_shim_locations().await.unwrap() {
                assert!(shim.path.exists());
            }
        }
    };
}

#[macro_export]
macro_rules! generate_download_install_tests {
    ($id:literal, $version:literal) => {
        generate_download_install_tests!($id, $version, None);
    };
    ($id:literal, $version:literal, $schema:expr) => {
        #[tokio::test(flavor = "multi_thread")]
        async fn downloads_verifies_installs_tool() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                sandbox.create_schema_plugin($id, schema).await
            } else {
                sandbox.create_plugin($id).await
            };
            let spec = UnresolvedVersionSpec::parse($version).unwrap();

            plugin
                .tool
                .setup(
                    &spec,
                    proto_pdk_test_utils::flow::install::InstallOptions::default(),
                )
                .await
                .unwrap();

            // Check install dir exists
            let base_dir = sandbox.proto_dir.join("tools").join($id).join($version);
            let tool_dir = plugin.tool.get_product_dir();

            assert_eq!(tool_dir, base_dir);
            assert!(base_dir.exists());

            // Check bin path exists (would panic)
            plugin.tool.locate_exe_file().await.unwrap();

            // Check things exist
            for bin in plugin.tool.resolve_bin_locations(true).await.unwrap() {
                assert!(bin.path.exists());
            }

            for shim in plugin.tool.resolve_shim_locations().await.unwrap() {
                assert!(shim.path.exists());
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn downloads_prebuilt_and_checksum_to_temp() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                sandbox.create_schema_plugin($id, schema).await
            } else {
                sandbox.create_plugin($id).await
            };
            let mut tool = plugin.tool;

            tool.set_version(VersionSpec::parse($version).unwrap());

            let temp_dir = tool.get_temp_dir();

            tool.install_from_prebuilt(
                &tool.get_product_dir(),
                &temp_dir,
                proto_pdk_test_utils::flow::install::InstallOptions::default(),
            )
            .await
            .unwrap();

            assert!(temp_dir.exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_install_if_already_installed() {
            if $version == "canary" {
                // Canary always overwrites instead of aborting
                return;
            }

            let sandbox = create_empty_proto_sandbox();
            let plugin = if let Some(schema) = $schema {
                sandbox.create_schema_plugin($id, schema).await
            } else {
                sandbox.create_plugin($id).await
            };
            let mut tool = plugin.tool;
            let spec = VersionSpec::parse($version).unwrap();

            // Fake the installation so we avoid downloading
            tool.set_version(spec.clone());
            tool.inventory.manifest.installed_versions.insert(spec);

            std::fs::create_dir_all(&tool.get_product_dir()).unwrap();

            assert!(!tool
                .install(proto_pdk_test_utils::flow::install::InstallOptions::default())
                .await
                .unwrap());
        }
    };
}

#[macro_export]
macro_rules! generate_resolve_versions_tests {
    ($id:literal, { $( $k:literal => $v:literal, )* }) => {
        generate_resolve_versions_tests!($id, { $( $k => $v, )* }, None);
    };
    ($id:literal, { $( $k:literal => $v:literal, )* }, $schema:expr) => {
        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_latest_alias() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                sandbox.create_schema_plugin($id, schema).await
            } else {
                sandbox.create_plugin($id).await
            };

            plugin.tool.resolve_version(
                &UnresolvedVersionSpec::parse("latest").unwrap(),
                false,
            ).await.unwrap();

            assert_ne!(plugin.tool.get_resolved_version(), "latest");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolve_version_or_alias() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                sandbox.create_schema_plugin($id, schema).await
            } else {
                sandbox.create_plugin($id).await
            };

            $(
                plugin.tool.resolve_version(
                    &UnresolvedVersionSpec::parse($k).unwrap(),
                    false,
                ).await.unwrap();

                assert_eq!(
                    plugin.tool.get_resolved_version(),
                    $v
                );
                plugin.tool.version = None;
            )*
        }

        // #[tokio::test(flavor = "multi_thread")]
        // async fn resolve_custom_alias() {
        //
        //     let sandbox = create_empty_proto_sandbox();

        //     sandbox.create_file(
        //         format!(".proto/tools/{}/manifest.json", $id),
        //         r#"{"aliases":{"example":"1.0.0"}}"#,
        //     );

        //     let mut plugin = sandbox.create_plugin($id).await;

        //     assert_eq!(
        //         plugin.tool.resolve_version("example").await.unwrap(),
        //         "1.0.0"
        //     );
        // }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Failed to resolve unknown to a valid supported version")]
        async fn errors_invalid_alias() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                sandbox.create_schema_plugin($id, schema).await
            } else {
                sandbox.create_plugin($id).await
            };

            plugin.tool.resolve_version(
                &UnresolvedVersionSpec::parse("unknown").unwrap(),
                false,
            ).await.unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Failed to resolve 99.99.99 to a valid supported version")]
        async fn errors_invalid_version() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                sandbox.create_schema_plugin($id, schema).await
            } else {
                sandbox.create_plugin($id).await
            };

            plugin.tool.resolve_version(
                &UnresolvedVersionSpec::parse("99.99.99").unwrap(),
                false,
            ).await.unwrap();
        }
    };
}

#[macro_export]
macro_rules! generate_shims_test {
    ($id:literal) => {
        generate_shims_test!($id, [$id]);
    };
    ($id:literal, [ $($bin:literal),* ]) => {
        generate_shims_test!($id, [ $($bin),* ], None);
    };
    ($id:literal, [ $($bin:literal),* ], $schema:expr) => {
        #[tokio::test(flavor = "multi_thread")]
        async fn creates_shims() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                sandbox.create_schema_plugin($id, schema).await
            } else {
                sandbox.create_plugin($id).await
            };

            plugin.tool.generate_shims(false).await.unwrap();

            $(
                assert!(
                    sandbox.proto_dir.join("shims").join(if cfg!(windows) {
                        format!("{}.exe", $bin)
                    } else {
                        $bin.to_string()
                    }).exists()
                );
            )*

            starbase_sandbox::assert_snapshot!(std::fs::read_to_string(
                sandbox.path().join(".proto/shims/registry.json")
            ).unwrap());
        }
    };
}
