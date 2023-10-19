#[macro_export]
macro_rules! generate_download_install_tests {
    ($id:literal, $version:literal) => {
        generate_download_install_tests!($id, $version, None);
    };
    ($id:literal, $version:literal, $schema:expr) => {
        #[tokio::test]
        async fn downloads_verifies_installs_tool() {
            let sandbox = starbase_sandbox::create_empty_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                create_schema_plugin($id, sandbox.path(), schema)
            } else {
                create_plugin($id, sandbox.path())
            };

            plugin
                .tool
                .setup(
                    &proto_pdk_test_utils::UnresolvedVersionSpec::parse($version).unwrap(),
                    false,
                )
                .await
                .unwrap();

            // Check install dir exists
            let base_dir = sandbox.path().join(".proto/tools").join($id).join($version);
            let tool_dir = plugin.tool.get_tool_dir();

            assert_eq!(tool_dir, base_dir);
            assert!(base_dir.exists());

            // Check bin path exists (would panic)
            plugin.tool.get_bin_path().unwrap();

            // Check global bin exists
            assert!(sandbox
                .path()
                .join(".proto/shims")
                .join(if cfg!(windows) {
                    format!("{}.cmd", $id)
                } else {
                    $id.into()
                })
                .exists());
        }

        #[tokio::test]
        async fn downloads_prebuilt_and_checksum_to_temp() {
            let sandbox = starbase_sandbox::create_empty_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                create_schema_plugin($id, sandbox.path(), schema)
            } else {
                create_plugin($id, sandbox.path())
            };
            let mut tool = plugin.tool;

            tool.version = Some(proto_pdk_test_utils::VersionSpec::parse($version).unwrap());

            let download_file = tool
                .install_from_prebuilt(&tool.get_tool_dir())
                .await
                .unwrap();

            assert!(download_file.starts_with(tool.get_temp_dir()));
        }

        #[tokio::test]
        async fn doesnt_install_if_already_installed() {
            if $version == "canary" {
                // Canary always overwrites instead of aborting
                return;
            }

            let sandbox = starbase_sandbox::create_empty_sandbox();
            let plugin = if let Some(schema) = $schema {
                create_schema_plugin($id, sandbox.path(), schema)
            } else {
                create_plugin($id, sandbox.path())
            };
            let mut tool = plugin.tool;
            let spec = proto_pdk_test_utils::VersionSpec::parse($version).unwrap();

            tool.version = Some(spec.clone());
            tool.manifest.installed_versions.insert(spec);

            std::fs::create_dir_all(&tool.get_tool_dir()).unwrap();

            assert!(!tool.install(false).await.unwrap());
        }
    };
}

#[macro_export]
macro_rules! generate_resolve_versions_tests {
    ($id:literal, { $( $k:literal => $v:literal, )* }) => {
        generate_resolve_versions_tests!($id, { $( $k => $v, )* }, None);
    };
    ($id:literal, { $( $k:literal => $v:literal, )* }, $schema:expr) => {
        #[tokio::test]
        async fn resolves_latest_alias() {
            let sandbox = starbase_sandbox::create_empty_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                create_schema_plugin($id, sandbox.path(), schema)
            } else {
                create_plugin($id, sandbox.path())
            };

            plugin.tool.resolve_version(
                &proto_pdk_test_utils::UnresolvedVersionSpec::parse("latest").unwrap(),
            ).await.unwrap();

            assert_ne!(plugin.tool.get_resolved_version(), "latest");
        }

        #[tokio::test]
        async fn resolve_version_or_alias() {
            let sandbox = starbase_sandbox::create_empty_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                create_schema_plugin($id, sandbox.path(), schema)
            } else {
                create_plugin($id, sandbox.path())
            };

            $(
                plugin.tool.resolve_version(
                    &proto_pdk_test_utils::UnresolvedVersionSpec::parse($k).unwrap(),
                ).await.unwrap();

                assert_eq!(
                    plugin.tool.get_resolved_version(),
                    $v
                );
                plugin.tool.version = None;
            )*
        }

        // #[tokio::test]
        // async fn resolve_custom_alias() {
        //
        //     let sandbox = starbase_sandbox::create_empty_sandbox();

        //     sandbox.create_file(
        //         format!(".proto/tools/{}/manifest.json", $id),
        //         r#"{"aliases":{"example":"1.0.0"}}"#,
        //     );

        //     let mut plugin = create_plugin($id, sandbox.path());

        //     assert_eq!(
        //         plugin.tool.resolve_version("example").await.unwrap(),
        //         "1.0.0"
        //     );
        // }

        #[tokio::test]
        #[should_panic(expected = "Failed to resolve a semantic version for unknown")]
        async fn errors_invalid_alias() {
            let sandbox = starbase_sandbox::create_empty_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                create_schema_plugin($id, sandbox.path(), schema)
            } else {
                create_plugin($id, sandbox.path())
            };

            plugin.tool.resolve_version(
                &proto_pdk_test_utils::UnresolvedVersionSpec::parse("unknown").unwrap(),
            ).await.unwrap();
        }

        #[tokio::test]
        #[should_panic(expected = "Failed to resolve a semantic version for 99.99.99")]
        async fn errors_invalid_version() {
            let sandbox = starbase_sandbox::create_empty_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                create_schema_plugin($id, sandbox.path(), schema)
            } else {
                create_plugin($id, sandbox.path())
            };

            plugin.tool.resolve_version(
                &proto_pdk_test_utils::UnresolvedVersionSpec::parse("99.99.99").unwrap(),
            ).await.unwrap();
        }
    };
}

#[macro_export]
macro_rules! generate_global_shims_test {
    ($id:literal) => {
        generate_global_shims_test!($id, []);
    };
    ($id:literal, [ $($bin:literal),* ]) => {
        generate_global_shims_test!($id, [ $($bin),* ], None);
    };
    ($id:literal, [ $($bin:literal),* ], $schema:expr) => {
        #[tokio::test]
        async fn creates_global_shims() {
            let sandbox = starbase_sandbox::create_empty_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                create_schema_plugin($id, sandbox.path(), schema)
            } else {
                create_plugin($id, sandbox.path())
            };

            plugin.tool.create_shims(false).await.unwrap();

            starbase_sandbox::assert_snapshot!(std::fs::read_to_string(
                sandbox.path().join(".proto/shims").join(if cfg!(windows) {
                    format!("{}.cmd", $id)
                } else {
                    $id.to_string()
                })
            ).unwrap());

            $(
                starbase_sandbox::assert_snapshot!(std::fs::read_to_string(
                    sandbox.path().join(".proto/shims").join(if cfg!(windows) {
                        format!("{}.cmd", $bin)
                    } else {
                        $bin.to_string()
                    })
                ).unwrap());
            )*
        }
    };
}

#[macro_export]
macro_rules! generate_local_shims_test {
    ($id:literal, [ $($bin:literal),* ]) => {
        generate_local_shims_test!($id, [ $($bin),* ], None);
    };
    ($id:literal, [ $($bin:literal),* ], $schema:expr) => {
        #[tokio::test]
        async fn creates_local_shims() {
            let sandbox = starbase_sandbox::create_empty_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                create_schema_plugin($id, sandbox.path(), schema)
            } else {
                create_plugin($id, sandbox.path())
            };

            plugin.tool.create_shims(false).await.unwrap();

            $(
                starbase_sandbox::assert_snapshot!(std::fs::read_to_string(
                    sandbox.path().join(".proto/tools").join($id).join("latest/shims").join(if cfg!(windows) {
                        format!("{}.ps1", $bin)
                    } else {
                        $bin.to_string()
                    })
                ).unwrap());
            )*
        }
    };
}

#[macro_export]
macro_rules! generate_globals_test {
    ($id:literal, $dep:literal) => {
        generate_globals_test!($id, $dep, None, None);
    };
    ($id:literal, $dep:literal, $env:literal) => {
        generate_globals_test!($id, $dep, Some($env.to_string()), None);
    };
    ($id:literal, $dep:literal, $env:expr, $schema:expr) => {
        #[tokio::test]
        async fn installs_and_uninstalls_globals() {
            let sandbox = starbase_sandbox::create_empty_sandbox();
            let mut plugin = if let Some(schema) = $schema {
                create_schema_plugin($id, sandbox.path(), schema)
            } else {
                create_plugin($id, sandbox.path())
            };

            let env_var: Option<String> = $env;

            if let Some(var) = &env_var {
                std::env::set_var(var.to_owned(), sandbox.path().to_string_lossy().to_string());
            }

            plugin.tool.locate_globals_dir().await.unwrap();

            let globals_dir = plugin
                .tool
                .get_globals_bin_dir()
                .expect("Globals directory required for testing!");

            let mut exts = if cfg!(windows) {
                vec![".exe", ".ps1", ".cmd"]
            } else {
                vec!["", ".sh"]
            };
            exts.extend(vec![".ts", ".js", ".mjs", ".cjs"]);

            let dep = $dep; // URL, path, or string

            let dep_name = if let Some(index) = dep.rfind("/") {
                &dep[index + 1..]
            } else {
                &dep
            };

            let dep_name_without_version = if let Some(index) = dep_name.find("@") {
                &dep_name[0..index]
            } else {
                &dep_name
            };

            // This is left in for debugging purposes
            dbg!(&globals_dir, &dep, &dep_name, &dep_name_without_version);
            sandbox.debug_files();

            plugin.tool.install_global(dep).await.unwrap();

            assert!(exts.iter().any(|ext| globals_dir
                .join(format!("{}{ext}", dep_name_without_version))
                .exists()
                || globals_dir
                    .join(format!("bin/{}{ext}", dep_name_without_version))
                    .exists()));

            plugin
                .tool
                .uninstall_global(dep_name_without_version)
                .await
                .unwrap();

            assert!(exts.iter().all(|ext| !globals_dir
                .join(format!("{}{ext}", dep_name_without_version))
                .exists()
                && !globals_dir
                    .join(format!("bin/{}{ext}", dep_name_without_version))
                    .exists()));

            if let Some(var) = &env_var {
                std::env::remove_var(var.to_owned());
            }
        }
    };
}
