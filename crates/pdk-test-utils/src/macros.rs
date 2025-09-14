#[macro_export]
macro_rules! create_plugin {
    ($sandbox:ident, $id:literal, $schema:expr, $factory:expr) => {
        if let Some(schema) = $schema {
            $sandbox
                .create_schema_plugin_with_config($id, schema, $factory)
                .await
        } else {
            $sandbox.create_plugin_with_config($id, $factory).await
        }
    };
}

#[macro_export]
macro_rules! check_install_success {
    ($plugin:ident) => {
        $plugin.tool.locate_exe_file().await.unwrap();

        assert!($plugin.tool.get_product_dir().exists());

        for bin in $plugin.tool.resolve_bin_locations(true).await.unwrap() {
            assert!(bin.path.exists());
        }

        for shim in $plugin.tool.resolve_shim_locations().await.unwrap() {
            assert!(shim.path.exists());
        }
    };
}

#[macro_export]
macro_rules! do_build_from_source {
    ($sandbox:ident, $plugin:ident, $spec:literal) => {
        let spec = ToolSpec::parse($spec).unwrap();
        let result = $plugin
            .tool
            .setup(
                &spec,
                flow::install::InstallOptions {
                    console: Some(ProtoConsole::new_testing()),
                    log_writer: Some(Default::default()),
                    strategy: InstallStrategy::BuildFromSource,
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
                    $sandbox
                        .path()
                        .join(format!("proto-{}-build.log", $plugin.tool.get_id()))
                )
                .unwrap()
            );
        }

        result.unwrap();

        check_install_success!($plugin);
    };
}

#[macro_export]
macro_rules! generate_build_install_tests {
    ($id:literal, $spec:literal) => {
        generate_build_install_tests!($id, $spec, None);
    };
    ($id:literal, $spec:literal, $schema:expr) => {
        generate_build_install_tests!($id, $spec, $schema, |_| {});
    };
    ($id:literal, $spec:literal, $schema:expr, $factory:expr) => {
        #[tokio::test(flavor = "multi_thread")]
        async fn builds_tool_from_source() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = create_plugin!(sandbox, $id, $schema, $factory);

            do_build_from_source!(sandbox, plugin, $spec);
        }
    };
}

#[macro_export]
macro_rules! do_install_prebuilt {
    ($sandbox:ident, $plugin:ident, $spec:literal) => {
        let spec = ToolSpec::parse($spec).unwrap();
        let result = $plugin
            .tool
            .setup(&spec, flow::install::InstallOptions::default())
            .await;

        check_install_success!($plugin);
    };
}

#[macro_export]
macro_rules! generate_download_install_tests {
    ($id:literal, $spec:literal) => {
        generate_download_install_tests!($id, $spec, None);
    };
    ($id:literal, $spec:literal, $schema:expr) => {
        generate_download_install_tests!($id, $spec, $schema, |_| {});
    };
    ($id:literal, $spec:literal, $schema:expr, $factory:expr) => {
        generate_native_install_tests!($id, $spec, $schema);

        #[tokio::test(flavor = "multi_thread")]
        async fn downloads_prebuilt_and_checksum_to_temp() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = create_plugin!(sandbox, $id, $schema, $factory);
            let mut tool = plugin.tool;

            tool.resolve_version(&ToolSpec::parse($spec).unwrap(), false)
                .await
                .unwrap();

            tool.install(flow::install::InstallOptions::default())
                .await
                .unwrap();

            assert!(tool.get_temp_dir().exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_install_if_already_installed() {
            if $spec == "canary" {
                // Canary always overwrites instead of aborting
                return;
            }

            let sandbox = create_empty_proto_sandbox();
            let mut plugin = create_plugin!(sandbox, $id, $schema, $factory);
            let mut tool = plugin.tool;
            let spec = VersionSpec::parse($spec).unwrap();

            // Fake the installation so we avoid downloading
            tool.set_version(spec.clone());
            tool.inventory.manifest.installed_versions.insert(spec);

            std::fs::create_dir_all(tool.get_product_dir()).unwrap();

            assert!(
                tool.install(flow::install::InstallOptions::default())
                    .await
                    .unwrap()
                    .is_none()
            );
        }
    };
}

#[macro_export]
macro_rules! generate_native_install_tests {
    ($id:literal, $spec:literal) => {
        generate_native_install_tests!($id, $spec, None);
    };
    ($id:literal, $spec:literal, $schema:expr) => {
        generate_native_install_tests!($id, $spec, $schema, |_| {});
    };
    ($id:literal, $spec:literal, $schema:expr, $factory:expr) => {
        #[tokio::test(flavor = "multi_thread")]
        async fn installs_tool() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = create_plugin!(sandbox, $id, $schema, $factory);

            do_install_prebuilt!(sandbox, plugin, $spec);
        }
    };
}

#[macro_export]
macro_rules! generate_resolve_versions_tests {
    ($id:literal, { $( $k:literal => $v:literal, )* }) => {
        generate_resolve_versions_tests!($id, { $( $k => $v, )* }, None);
    };
    ($id:literal, { $( $k:literal => $v:literal, )* }, $schema:expr) => {
        generate_resolve_versions_tests!($id, { $( $k => $v, )* }, $schema, |_| {});
    };
    ($id:literal, { $( $k:literal => $v:literal, )* }, $schema:expr, $factory:expr) => {
        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_latest_alias() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = create_plugin!(sandbox, $id, $schema, $factory);

            plugin.tool.resolve_version(
                &ToolSpec::parse("latest").unwrap(),
                false,
            ).await.unwrap();

            assert_ne!(plugin.tool.get_resolved_version(), "latest");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolve_version_or_alias() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = create_plugin!(sandbox, $id, $schema, $factory);

            $(
                plugin.tool.resolve_version(
                    &ToolSpec::parse($k).unwrap(),
                    false,
                ).await.unwrap();

                assert_eq!(
                    plugin.tool.get_resolved_version(),
                    $v
                );
                plugin.tool.version = None;
            )*
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "FailedVersionResolve")]
        async fn errors_invalid_alias() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = create_plugin!(sandbox, $id, $schema, $factory);

            plugin.tool.resolve_version(
                &ToolSpec::parse("unknown").unwrap(),
                false,
            ).await.unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "FailedVersionResolve")]
        async fn errors_invalid_version() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = create_plugin!(sandbox, $id, $schema, $factory);

            plugin.tool.resolve_version(
                &ToolSpec::parse("99.99.99").unwrap(),
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
        generate_shims_test!($id, [ $($bin),* ], $schema, |_| {});
    };
    ($id:literal, [ $($bin:literal),* ], $schema:expr, $factory:expr) => {
        #[tokio::test(flavor = "multi_thread")]
        async fn creates_shims() {
            let sandbox = create_empty_proto_sandbox();
            let mut plugin = create_plugin!(sandbox, $id, $schema, $factory);

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
