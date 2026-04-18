use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use starbase_sandbox::{Sandbox, create_empty_sandbox, locate_fixture};
use starbase_utils::fs;
use std::path::PathBuf;
use std::time::Duration;
use warpgate::{
    DataLocator, FileLocator, GitHubLocator, Id, PluginLoader, PluginLocator, UrlLocator,
    hash_sha256,
};

// A pinned, stable .wasm release to use across URL-based tests.
const SYSTEM_TOOLCHAIN_URL: &str = "https://github.com/moonrepo/plugins/releases/download/system_toolchain-v1.0.0/system_toolchain.wasm";

fn create_loader() -> (Sandbox, PluginLoader) {
    let sandbox = create_empty_sandbox();
    let loader = PluginLoader::new(sandbox.path().join("plugins"), sandbox.path().join("temp"));

    (sandbox, loader)
}

// Returns the expected cache path for a URL download with id="test".
fn url_cache_path(loader: &PluginLoader, url: &str) -> PathBuf {
    loader.create_cache_path(&Id::raw("test"), &hash_sha256(url), ".wasm", false)
}

mod loader {
    use super::*;

    // -------------------------------------------------------------------------
    // Data / blob locator
    // -------------------------------------------------------------------------

    mod data {
        use super::*;

        fn make_locator(wasm: &[u8]) -> PluginLocator {
            PluginLocator::Data(Box::new(DataLocator {
                data: format!("data://{}", BASE64_STANDARD.encode(wasm)),
                bytes: None,
            }))
        }

        #[tokio::test]
        async fn decodes_bytes() {
            let (sandbox, loader) = create_loader();
            let fixture = locate_fixture("loader");
            let wasm = fs::read_file_bytes(fixture.join("test.wasm")).unwrap();

            let path = loader
                .load_plugin(Id::raw("test"), make_locator(&wasm))
                .await
                .unwrap();

            assert_eq!(
                path,
                sandbox.path().join(
                    "plugins/test-fb04dcb6970e4c3d1873de51fd5a50d7bb46b3383113602665c350ec40b5f990.wasm"
                )
            );
        }

        // Second call with identical blob data must return the same path from
        // cache without re-writing the file (modification time stays the same).
        #[tokio::test]
        async fn uses_cache_on_second_call() {
            let (_sandbox, loader) = create_loader();
            let fixture = locate_fixture("loader");
            let wasm = fs::read_file_bytes(fixture.join("test.wasm")).unwrap();

            let path1 = loader
                .load_plugin(Id::raw("test"), make_locator(&wasm))
                .await
                .unwrap();

            assert!(path1.exists());

            let mtime1 = path1.metadata().unwrap().modified().unwrap();

            let path2 = loader
                .load_plugin(Id::raw("test"), make_locator(&wasm))
                .await
                .unwrap();

            assert_eq!(path1, path2);

            let mtime2 = path2.metadata().unwrap().modified().unwrap();

            assert_eq!(mtime1, mtime2);
        }

        // Concurrent calls with the same blob data must all resolve to the same
        // path without errors, and the in-process lock must prevent duplicate writes.
        #[tokio::test]
        async fn concurrent_calls_return_same_path() {
            let (_sandbox, loader) = create_loader();
            let fixture = locate_fixture("loader");
            let wasm = fs::read_file_bytes(fixture.join("test.wasm")).unwrap();

            let (r1, r2, r3) = tokio::join!(
                loader.load_plugin(Id::raw("test"), make_locator(&wasm)),
                loader.load_plugin(Id::raw("test"), make_locator(&wasm)),
                loader.load_plugin(Id::raw("test"), make_locator(&wasm)),
            );

            let path1 = r1.unwrap();
            let path2 = r2.unwrap();
            let path3 = r3.unwrap();

            assert!(path1.exists());
            assert_eq!(path1, path2);
            assert_eq!(path1, path3);
        }
    }

    // -------------------------------------------------------------------------
    // File locator
    // -------------------------------------------------------------------------

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

    // -------------------------------------------------------------------------
    // URL locator
    // -------------------------------------------------------------------------

    mod source_url {
        use super::*;

        fn make_locator(url: &str) -> PluginLocator {
            PluginLocator::Url(Box::new(UrlLocator { url: url.into() }))
        }

        // -- Error cases ------------------------------------------------------

        #[tokio::test]
        #[should_panic(expected = "NotFound")]
        async fn errors_broken_url() {
            let (_sandbox, loader) = create_loader();

            loader
                .load_plugin(
                    Id::raw("test"),
                    make_locator("https://github.com/moonrepo/deno-plugin/releases/download/v0.0.2/deno_plugin_invalid_name.wasm"),
                )
                .await
                .unwrap();
        }

        // When offline and the plugin is not yet cached, load_plugin must fail
        // with a RequiredInternetConnection error rather than hanging or panicking.
        #[tokio::test]
        #[should_panic(expected = "RequiredInternetConnection")]
        async fn offline_errors_when_no_cache() {
            let (_sandbox, mut loader) = create_loader();
            loader.set_offline_checker(|| true);

            loader
                .load_plugin(Id::raw("test"), make_locator(SYSTEM_TOOLCHAIN_URL))
                .await
                .unwrap();
        }

        // -- Download basics --------------------------------------------------

        #[tokio::test]
        async fn downloads_to_plugins() {
            let (sandbox, loader) = create_loader();

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    make_locator("https://github.com/moonrepo/deno-plugin/releases/download/v0.0.2/deno_plugin.wasm"),
                )
                .await
                .unwrap();

            assert_eq!(
                path,
                sandbox.path().join(
                    "plugins/test-bf1e7cdc7ca22a4b75e10560bfce659bd51125f2e64bad2143633d20e30e9e01.wasm"
                )
            );
        }

        #[tokio::test]
        async fn supports_latest() {
            let (sandbox, loader) = create_loader();

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    make_locator("https://github.com/moonrepo/deno-plugin/releases/latest/download/deno_plugin.wasm"),
                )
                .await
                .unwrap();

            assert_eq!(
                path,
                sandbox.path().join(
                    "plugins/test-latest-07bced931a4b253ba3da8836c0510275512ce09436c63ed44b77b3683effae91.wasm"
                )
            );
        }

        // Downloaded file must be a valid, non-empty .wasm binary.
        #[tokio::test]
        async fn downloads_valid_wasm() {
            let (_sandbox, loader) = create_loader();

            let path = loader
                .load_plugin(Id::raw("test"), make_locator(SYSTEM_TOOLCHAIN_URL))
                .await
                .unwrap();

            assert!(path.exists());
            assert!(path.extension().is_some_and(|e| e == "wasm"));

            let bytes = fs::read_file_bytes(&path).unwrap();

            // All WASM binaries begin with the 4-byte magic header `\0asm`.
            assert!(bytes.starts_with(b"\0asm"));
        }

        // -- Cache behaviour --------------------------------------------------

        // The second call with the same URL must return the same path from the
        // local cache without triggering a network request (modification time
        // must be unchanged).
        #[tokio::test]
        async fn uses_cache_on_second_call() {
            let (_sandbox, loader) = create_loader();

            let path1 = loader
                .load_plugin(Id::raw("test"), make_locator(SYSTEM_TOOLCHAIN_URL))
                .await
                .unwrap();

            assert!(path1.exists());

            let mtime1 = path1.metadata().unwrap().modified().unwrap();

            let path2 = loader
                .load_plugin(Id::raw("test"), make_locator(SYSTEM_TOOLCHAIN_URL))
                .await
                .unwrap();

            assert_eq!(path1, path2);

            let mtime2 = path2.metadata().unwrap().modified().unwrap();

            assert_eq!(mtime1, mtime2);
        }

        // With zero cache duration the loader treats any cached file as stale
        // and calls download_plugin. However, when offline the stale file must
        // still be returned rather than failing.
        #[tokio::test]
        async fn offline_uses_stale_cache() {
            let (sandbox, mut loader) = create_loader();

            // Pre-create the cache file to simulate a previous download.
            let cache_path = url_cache_path(&loader, SYSTEM_TOOLCHAIN_URL);
            fs::create_dir_all(sandbox.path().join("plugins")).unwrap();
            fs::write_file(&cache_path, b"\0asm fake cached wasm").unwrap();

            // Zero duration means the file is always "stale" when online,
            // but the offline override should rescue it.
            loader.set_cache_duration(Duration::ZERO);
            loader.set_offline_checker(|| true);

            let path = loader
                .load_plugin(Id::raw("test"), make_locator(SYSTEM_TOOLCHAIN_URL))
                .await
                .unwrap();

            assert_eq!(path, cache_path);
        }

        // -- In-process locking behaviour -------------------------------------

        // When multiple async tasks concurrently request the same URL, they
        // must all receive the same valid path, and only one actual download
        // should be triggered (the rest skip via the in-process Mutex).
        #[tokio::test]
        async fn concurrent_calls_return_same_path() {
            let (_sandbox, loader) = create_loader();

            let (r1, r2, r3) = tokio::join!(
                loader.load_plugin(Id::raw("test"), make_locator(SYSTEM_TOOLCHAIN_URL)),
                loader.load_plugin(Id::raw("test"), make_locator(SYSTEM_TOOLCHAIN_URL)),
                loader.load_plugin(Id::raw("test"), make_locator(SYSTEM_TOOLCHAIN_URL)),
            );

            let path1 = r1.unwrap();
            let path2 = r2.unwrap();
            let path3 = r3.unwrap();

            assert!(path1.exists());
            assert_eq!(path1, path2);
            assert_eq!(path1, path3);
        }

        // If a competing process (or prior call) already wrote the destination
        // file between the outer is_cached check and the inner lock acquisition,
        // download_plugin must exit early without overwriting or re-downloading.
        // We simulate this by pre-populating dest_file and forcing a zero cache
        // duration so that is_cached → false and download_plugin is entered.
        #[tokio::test]
        async fn skips_download_when_dest_exists_after_acquiring_lock() {
            let (sandbox, mut loader) = create_loader();

            let cache_path = url_cache_path(&loader, SYSTEM_TOOLCHAIN_URL);
            fs::create_dir_all(sandbox.path().join("plugins")).unwrap();

            // Write a sentinel payload — a real download would overwrite this
            // with the actual WASM binary.
            let sentinel = b"sentinel - should not be overwritten";
            fs::write_file(&cache_path, sentinel).unwrap();

            // Zero duration bypasses is_cached so download_plugin is invoked,
            // but it must see dest_file.exists() == true and return early.
            loader.set_cache_duration(Duration::ZERO);

            let path = loader
                .load_plugin(Id::raw("test"), make_locator(SYSTEM_TOOLCHAIN_URL))
                .await
                .unwrap();

            assert_eq!(path, cache_path);

            let written = fs::read_file_bytes(&cache_path).unwrap();

            assert_eq!(written, sentinel);
        }

        // After a successful download the per-URL temp file must be removed.
        // The temp subdirectory (keyed by the URL hash) may remain but must
        // contain no leftover files.
        #[tokio::test]
        async fn cleans_temp_file_after_download() {
            let (sandbox, loader) = create_loader();

            loader
                .load_plugin(Id::raw("test"), make_locator(SYSTEM_TOOLCHAIN_URL))
                .await
                .unwrap();

            let temp_url_dir = sandbox
                .path()
                .join("temp")
                .join(hash_sha256(SYSTEM_TOOLCHAIN_URL));

            if temp_url_dir.exists() {
                let leftover: Vec<_> = temp_url_dir
                    .read_dir()
                    .unwrap()
                    .filter_map(|e| e.ok())
                    .collect();

                assert!(
                    leftover.is_empty(),
                    "temp files were not cleaned up after download: {:?}",
                    leftover.iter().map(|e| e.path()).collect::<Vec<_>>()
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // GitHub locator
    // -------------------------------------------------------------------------

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

            assert_eq!(
                path,
                sandbox.path().join(
                    "plugins/test-d38b431dfda1d53f9d1bc97ea409a97dcaa03efa11a508d73f1f9497b9bcaa3a.wasm"
                )
            );
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

            assert_eq!(
                path,
                sandbox.path().join(
                    "plugins/test-latest-171d1fba97b9fe0489125b9253b8fdb8a85d37837f966aaacd06df662292f507.wasm"
                )
            );
        }
    }
}
