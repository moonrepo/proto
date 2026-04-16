use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use starbase_sandbox::{Sandbox, create_empty_sandbox, locate_fixture};
use starbase_utils::fs;
use std::path::PathBuf;
use warpgate::{
    DataLocator, FileLocator, GitHubLocator, Id, PluginLoader, PluginLocator, UrlLocator,
};

fn create_loader() -> (Sandbox, PluginLoader) {
    let sandbox = create_empty_sandbox();
    let loader = PluginLoader::new(sandbox.path().join("plugins"), sandbox.path().join("temp"));

    (sandbox, loader)
}

mod loader {
    use super::*;

    mod data {
        use super::*;

        #[tokio::test]
        async fn decodes_bytes() {
            let (sandbox, loader) = create_loader();
            let fixture = locate_fixture("loader");
            let wasm = fs::read_file_bytes(fixture.join("test.wasm")).unwrap();

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::Data(Box::new(DataLocator {
                        data: format!("data://{}", BASE64_STANDARD.encode(wasm)),
                        bytes: None,
                    })),
                )
                .await
                .unwrap();

            assert_eq!(path, sandbox.path().join("plugins/test-fb04dcb6970e4c3d1873de51fd5a50d7bb46b3383113602665c350ec40b5f990.wasm"));
        }
    }

    mod source_file {
        use super::*;

        #[tokio::test]
        #[should_panic(expected = "MissingSourceFile")]
        async fn errors_missing_file() {
            let (_sandbox, loader) = create_loader();

            loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::File(Box::new(FileLocator {
                        file: "".into(),
                        path: Some(PathBuf::from("fake-file")),
                    })),
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
                    PluginLocator::File(Box::new(FileLocator {
                        file: "".into(),
                        path: Some(fixture.join("test.wasm")),
                    })),
                )
                .await
                .unwrap();

            assert_eq!(path, fixture.join("test.wasm"));
        }
    }

    mod source_url {
        use super::*;

        #[tokio::test]
        #[should_panic(expected = "NotFound")]
        async fn errors_broken_url() {
            let (_sandbox, loader) = create_loader();

            loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::Url(Box::new(UrlLocator { url: "https://github.com/moonrepo/deno-plugin/releases/download/v0.0.2/deno_plugin_invalid_name.wasm".into() })),
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
                    PluginLocator::Url(Box::new(UrlLocator { url: "https://github.com/moonrepo/deno-plugin/releases/download/v0.0.2/deno_plugin.wasm".into() })),
                )
                .await
                .unwrap();

            assert_eq!(path, sandbox.path().join("plugins/test-bf1e7cdc7ca22a4b75e10560bfce659bd51125f2e64bad2143633d20e30e9e01.wasm"));
        }

        #[tokio::test]
        async fn supports_latest() {
            let (sandbox, loader) = create_loader();

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::Url(Box::new(UrlLocator { url: "https://github.com/moonrepo/deno-plugin/releases/latest/download/deno_plugin.wasm".into() })),
                )
                .await
                .unwrap();

            assert_eq!(path, sandbox.path().join("plugins/test-latest-07bced931a4b253ba3da8836c0510275512ce09436c63ed44b77b3683effae91.wasm"));
        }
    }

    mod github {
        use super::*;

        #[tokio::test]
        #[should_panic(expected = "MissingGitHubAsset")]
        async fn errors_invalid_slug() {
            let (_sandbox, loader) = create_loader();

            loader
                .load_plugin(
                    Id::raw("test"),
                    PluginLocator::GitHub(Box::new(GitHubLocator {
                        repo_slug: "moonrepo/invalid-repo".into(),
                        tag: None,
                        project_name: None,
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
                        project_name: None,
                    })),
                )
                .await
                .unwrap();

            assert_eq!(path, sandbox.path().join("plugins/test-d38b431dfda1d53f9d1bc97ea409a97dcaa03efa11a508d73f1f9497b9bcaa3a.wasm"));
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
                        project_name: None,
                    })),
                )
                .await
                .unwrap();

            assert_eq!(path, sandbox.path().join("plugins/test-latest-171d1fba97b9fe0489125b9253b8fdb8a85d37837f966aaacd06df662292f507.wasm"));
        }
    }
}
