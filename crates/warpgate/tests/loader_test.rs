use starbase_sandbox::{create_empty_sandbox, locate_fixture, Sandbox};
use std::path::PathBuf;
use warpgate::{GitHubLocator, Id, PluginLoader, PluginLocator};

fn create_loader() -> (Sandbox, PluginLoader) {
    let sandbox = create_empty_sandbox();
    let loader = PluginLoader::new(sandbox.path().join("plugins"), sandbox.path().join("temp"));

    (sandbox, loader)
}

mod loader {
    use super::*;

    mod source_file {
        use super::*;

        #[tokio::test]
        #[should_panic(expected = "Cannot load test plugin, source file fake-file does not exist.")]
        async fn errors_missing_file() {
            let (_sandbox, loader) = create_loader();

            loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::File {
                        file: "".into(),
                        path: Some(PathBuf::from("fake-file")),
                    },
                )
                .await
                .unwrap();
        }

        #[tokio::test]
        async fn returns_path_asis() {
            let (_sandbox, loader) = create_loader();
            let fixture = locate_fixture("loader");

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::File {
                        file: "".into(),
                        path: Some(fixture.join("test.wasm")),
                    },
                )
                .await
                .unwrap();

            // Path is UNC prefixed
            if cfg!(windows) {
                assert!(path.ends_with("loader\\test.wasm"));
            } else {
                assert_eq!(path, fixture.join("test.wasm"));
            }
        }
    }

    mod source_url {
        use super::*;

        #[tokio::test]
        #[should_panic(expected = "does not exist")]
        async fn errors_broken_url() {
            let (_sandbox, loader) = create_loader();

            loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::Url { url: "https://github.com/moonrepo/deno-plugin/releases/download/v0.0.2/deno_plugin_invalid_name.wasm".into() },
                )
                .await
                .unwrap();
        }

        #[tokio::test]
        async fn downloads_to_plugins() {
            let (sandbox, loader) = create_loader();

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::Url { url: "https://github.com/moonrepo/deno-plugin/releases/download/v0.0.2/deno_plugin.wasm".into() },
                )
                .await
                .unwrap();

            assert_eq!(path, sandbox.path().join("plugins/test-1cab19a12ec96a1036dc5d51011634dddfa2911941f31e4957d7780bb70f88f0.wasm"));
        }

        #[tokio::test]
        async fn supports_latest() {
            let (sandbox, loader) = create_loader();

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::Url { url: "https://github.com/moonrepo/deno-plugin/releases/latest/download/deno_plugin.wasm".into() },
                )
                .await
                .unwrap();

            assert_eq!(path, sandbox.path().join("plugins/test-latest-db3f668c2fe22a7f9a6ce86b6fa8feeffbfd8e7874bdb854e82b154319675269.wasm"));
        }
    }

    mod github {
        use super::*;

        #[tokio::test]
        #[should_panic(
            expected = "Cannot download test plugin from GitHub (moonrepo/invalid-repo)"
        )]
        async fn errors_invalid_slug() {
            let (_sandbox, loader) = create_loader();

            loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::GitHub(Box::new(GitHubLocator {
                        repo_slug: "moonrepo/invalid-repo".into(),
                        tag: None,
                        tag_prefix: None,
                    })),
                )
                .await
                .unwrap();
        }

        #[tokio::test]
        async fn downloads_to_plugins() {
            let (sandbox, loader) = create_loader();

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::GitHub(Box::new(GitHubLocator {
                        repo_slug: "moonrepo/bun-plugin".into(),
                        tag: Some("v0.0.3".into()),
                        tag_prefix: None,
                    })),
                )
                .await
                .unwrap();

            assert_eq!(path, sandbox.path().join("plugins/test-3659b10975b8c1f704254f47c17e93f76abf6878dfcab9f9b6346491cf5b5df1.wasm"));
        }

        #[tokio::test]
        async fn supports_latest() {
            let (sandbox, loader) = create_loader();

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::GitHub(Box::new(GitHubLocator {
                        repo_slug: "moonrepo/bun-plugin".into(),
                        tag: None,
                        tag_prefix: None,
                    })),
                )
                .await
                .unwrap();

            assert_eq!(path, sandbox.path().join("plugins/test-latest-3659b10975b8c1f704254f47c17e93f76abf6878dfcab9f9b6346491cf5b5df1.wasm"));
        }
    }
}
