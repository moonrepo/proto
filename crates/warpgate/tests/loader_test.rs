use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use starbase_sandbox::{Sandbox, create_empty_sandbox, locate_fixture};
use starbase_utils::fs;
use std::path::PathBuf;
use std::time::Duration;
use warpgate::{
    DataLocator, FileLocator, GitHubLocator, Id, PluginLoader, PluginLocator, RegistryConfig,
    RegistryLocator, UrlLocator, hash_sha256,
};

// A pinned, stable .wasm release to use across URL-based tests.
const SYSTEM_TOOLCHAIN_URL: &str = "https://github.com/moonrepo/plugins/releases/download/system_toolchain-v1.0.0/system_toolchain.wasm";

fn create_loader() -> (Sandbox, PluginLoader) {
    let sandbox = create_empty_sandbox();
    let loader = PluginLoader::new(sandbox.path().join("plugins"), sandbox.path().join("temp"));

    (sandbox, loader)
}

fn create_loader_with_registries(registries: Vec<RegistryConfig>) -> (Sandbox, PluginLoader) {
    let (sandbox, mut loader) = create_loader();
    loader.add_registries(registries);

    (sandbox, loader)
}

// Returns the expected cache path for a URL download with id="test".
fn url_cache_path(loader: &PluginLoader, url: &str) -> PathBuf {
    loader.create_cache_path(&Id::raw("test"), &hash_sha256(url), "wasm", false)
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
                    "plugins/test-95e3b392be0793ad655146aa16c567723d9b653011bfc73d19d72b85fec010f6.wasm"
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

        // The cache file extension was previously hardcoded to `.wasm` on
        // lookup, which silently re-fetched any plugin persisted under a
        // different extension (OCI TOML/YAML/JSON layers, primarily). Once
        // a non-WASM extension is registered via `add_extensions`, a
        // pre-existing `.toml` cache file with a matching hash must be
        // adopted as-is, without writing a fresh `.wasm` alongside.
        #[tokio::test]
        async fn cache_probe_finds_non_wasm_extension() {
            let (_sandbox, mut loader) = create_loader();
            loader.add_extensions(vec!["toml".into()]);

            let bytes = b"fake-plugin-payload".to_vec();
            let hash = hash_sha256(&bytes);

            let toml_path = loader.create_cache_path(&Id::raw("test"), &hash, "toml", false);
            fs::create_dir_all(toml_path.parent().unwrap()).unwrap();
            fs::write_file(&toml_path, b"# pre-existing toml plugin").unwrap();

            let locator = PluginLocator::Data(Box::new(DataLocator {
                data: String::new(),
                bytes: Some(bytes),
            }));

            let result = loader.load_plugin(Id::raw("test"), locator).await.unwrap();

            assert_eq!(result, toml_path);

            // The probe must short-circuit before the Data loader runs, so
            // no fresh `.wasm` is written.
            let wasm_path = loader.create_cache_path(&Id::raw("test"), &hash, "wasm", false);
            assert!(!wasm_path.exists());
        }

        // When the same plugin happens to exist under multiple supported
        // extensions, the probe must return the first one in `extensions`
        // (which defaults to `wasm`-first). This pins the canonical-preference
        // ordering so a stray non-WASM file can never shadow a valid WASM.
        #[tokio::test]
        async fn cache_probe_prefers_first_extension() {
            let (_sandbox, mut loader) = create_loader();
            loader.add_extensions(vec!["toml".into()]);

            let bytes = b"fake-plugin-payload".to_vec();
            let hash = hash_sha256(&bytes);

            let wasm_path = loader.create_cache_path(&Id::raw("test"), &hash, "wasm", false);
            let toml_path = loader.create_cache_path(&Id::raw("test"), &hash, "toml", false);
            fs::create_dir_all(wasm_path.parent().unwrap()).unwrap();
            fs::write_file(&wasm_path, b"\0asm fake wasm").unwrap();
            fs::write_file(&toml_path, b"# fake toml").unwrap();

            let locator = PluginLocator::Data(Box::new(DataLocator {
                data: String::new(),
                bytes: Some(bytes),
            }));

            let result = loader.load_plugin(Id::raw("test"), locator).await.unwrap();

            assert_eq!(result, wasm_path);
        }

        // Without an explicit `add_extensions`, the probe is constrained to
        // `wasm` only. A pre-existing `.toml` file with a matching hash must
        // be ignored, and the loader must fall through to its normal save
        // path (producing a fresh `.wasm`).
        #[tokio::test]
        async fn cache_probe_ignores_unregistered_extension() {
            let (_sandbox, loader) = create_loader();
            // Default extensions: ["wasm"]

            let bytes = b"fake-plugin-payload".to_vec();
            let hash = hash_sha256(&bytes);

            let toml_path = loader.create_cache_path(&Id::raw("test"), &hash, "toml", false);
            fs::create_dir_all(toml_path.parent().unwrap()).unwrap();
            fs::write_file(&toml_path, b"# orphan toml - must be ignored").unwrap();

            let locator = PluginLocator::Data(Box::new(DataLocator {
                data: String::new(),
                bytes: Some(bytes),
            }));

            let result = loader.load_plugin(Id::raw("test"), locator).await.unwrap();

            let wasm_path = loader.create_cache_path(&Id::raw("test"), &hash, "wasm", false);
            assert_eq!(result, wasm_path);
            assert!(wasm_path.exists());
            // The orphan `.toml` remains untouched — the loader doesn't sweep
            // unrelated cache entries.
            assert!(toml_path.exists());
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
                    "plugins/test-1cab19a12ec96a1036dc5d51011634dddfa2911941f31e4957d7780bb70f88f0.wasm"
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
                    "plugins/test-latest-db3f668c2fe22a7f9a6ce86b6fa8feeffbfd8e7874bdb854e82b154319675269.wasm"
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
                    "plugins/test-0310ea7f16dfdf46d2b5650962d5b0906a14760a691fd1d6f2c32af321c595ef.wasm"
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
                    "plugins/test-latest-a819971be8d7e966f73c769fdca026ce01fac282e58c55a87a041ca71957e393.wasm"
                )
            );
        }

        // The rewrite's headline goal: once a GitHub plugin is cached, the
        // GitHub releases/tags APIs must never be touched again. We exercise
        // this with a deliberately unreachable repo slug — any actual network
        // attempt would error with MissingGitHubAsset or a transport error.
        // The pre-populated cache forces the probe to short-circuit before
        // load() ever runs, so `unwrap()` succeeding proves no API call fired.
        #[tokio::test]
        async fn cached_plugin_skips_github_api() {
            let (_sandbox, loader) = create_loader();

            let locator = PluginLocator::GitHub(Box::new(GitHubLocator {
                repo_slug: "warpgate-test-nonexistent/this-repo-must-never-exist".into(),
                tag: Some("v0.0.0-bogus".into()),
                project_name: None,
            }));

            let hash = hash_sha256(locator.to_string());
            let cache_path = loader.create_cache_path(&Id::raw("test"), &hash, "wasm", false);
            fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
            fs::write_file(&cache_path, b"\0asm fake wasm").unwrap();

            let result = loader.load_plugin(Id::raw("test"), locator).await.unwrap();

            assert_eq!(result, cache_path);
        }
    }

    // -------------------------------------------------------------------------
    // Registry (OCI) locator
    // -------------------------------------------------------------------------

    mod registry {
        use super::*;

        // A pinned, anonymous-pullable image on GHCR. This is one of the
        // built-in plugins shipped in `crates/core/src/config.rs`, so it must
        // remain available for the project to function.
        const FIXTURE_IMAGE: &str = "node_tool";
        const FIXTURE_TAG: &str = "0.17.9";
        const FIXTURE_HOST: &str = "ghcr.io";
        const FIXTURE_NAMESPACE: &str = "moonrepo";

        fn make_locator(
            registry: Option<&str>,
            namespace: Option<&str>,
            image: &str,
            tag: Option<&str>,
        ) -> PluginLocator {
            PluginLocator::Registry(Box::new(RegistryLocator {
                registry: registry.map(Into::into),
                namespace: namespace.map(Into::into),
                image: image.into(),
                tag: tag.map(Into::into),
            }))
        }

        fn assert_wasm(path: &std::path::Path) {
            assert!(path.exists(), "expected plugin path to exist: {path:?}");

            let bytes = fs::read_file_bytes(path).unwrap();

            // All WASM binaries begin with the 4-byte magic header `\0asm`.
            assert!(
                bytes.starts_with(b"\0asm"),
                "expected WASM magic header at {path:?}",
            );
        }

        // -- Step 1: locator host matches a configured registry --------------

        // When the locator's host AND namespace match a configured registry,
        // pull_image is called with that config (fallthrough = true) and the
        // blob is returned successfully.
        #[tokio::test]
        async fn matches_configured_registry() {
            let (_sandbox, loader) = create_loader_with_registries(vec![RegistryConfig {
                auth: false,
                default: false,
                registry: FIXTURE_HOST.into(),
                namespace: Some(FIXTURE_NAMESPACE.into()),
            }]);

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    make_locator(
                        Some(FIXTURE_HOST),
                        Some(FIXTURE_NAMESPACE),
                        FIXTURE_IMAGE,
                        Some(FIXTURE_TAG),
                    ),
                )
                .await
                .unwrap();

            assert_wasm(&path);
        }

        // -- Step 2: locator host without a matching config ------------------

        // When the locator has a host but no registry config matches, the
        // loader synthesizes an explicit `RegistryConfig` (auth = true) and
        // tries it. With an anonymous-pullable image, this still resolves.
        #[tokio::test]
        async fn explicit_host_no_matching_config_uses_explicit_fallback() {
            let (_sandbox, loader) = create_loader_with_registries(vec![]);

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    make_locator(
                        Some(FIXTURE_HOST),
                        Some(FIXTURE_NAMESPACE),
                        FIXTURE_IMAGE,
                        Some(FIXTURE_TAG),
                    ),
                )
                .await
                .unwrap();

            assert_wasm(&path);
        }

        // -- Step 3: locator without a host, iterate configured registries --

        // With no host on the locator, Steps 1 & 2 are skipped and the final
        // for-loop tries each configured registry.
        #[tokio::test]
        async fn no_host_iterates_configured_registries() {
            let (_sandbox, loader) = create_loader_with_registries(vec![RegistryConfig {
                auth: false,
                default: false,
                registry: FIXTURE_HOST.into(),
                namespace: Some(FIXTURE_NAMESPACE.into()),
            }]);

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    make_locator(None, None, FIXTURE_IMAGE, Some(FIXTURE_TAG)),
                )
                .await
                .unwrap();

            assert_wasm(&path);
        }

        // -- Step 4: error path (no host, no registries) ---------------------

        // When the locator has no host and no registries are configured,
        // both early branches are skipped and the final for-loop is empty,
        // so the loader returns OCIReferenceError without making any network
        // requests.
        #[tokio::test]
        #[should_panic(expected = "No valid registry or layer found for node_tool.")]
        async fn no_registries_no_host_returns_oci_reference_error() {
            let (_sandbox, loader) = create_loader_with_registries(vec![]);

            loader
                .load_plugin(
                    Id::raw("test"),
                    make_locator(None, None, FIXTURE_IMAGE, Some(FIXTURE_TAG)),
                )
                .await
                .unwrap();
        }

        // -- Tag / cache behaviour -------------------------------------------

        // A locator with no tag is treated as the `latest` tag, which is
        // reflected in the cache path by the `-latest-` segment.
        #[tokio::test]
        async fn uses_latest_when_tag_is_none() {
            let (sandbox, loader) = create_loader_with_registries(vec![]);

            let path = loader
                .load_plugin(
                    Id::raw("test"),
                    make_locator(
                        Some(FIXTURE_HOST),
                        Some(FIXTURE_NAMESPACE),
                        FIXTURE_IMAGE,
                        None,
                    ),
                )
                .await
                .unwrap();

            assert!(path.exists());

            let file_name = path.file_name().unwrap().to_string_lossy().to_string();
            assert!(
                file_name.contains("-latest-"),
                "expected cache filename to include `-latest-`, got {file_name}"
            );
            assert!(path.starts_with(sandbox.path().join("plugins")));
        }

        // After a successful pull, a second call with the same locator must
        // return the same path from cache without re-downloading (mtime is
        // unchanged).
        #[tokio::test]
        async fn uses_cache_on_second_call() {
            let (_sandbox, loader) = create_loader_with_registries(vec![RegistryConfig {
                auth: false,
                default: false,
                registry: FIXTURE_HOST.into(),
                namespace: Some(FIXTURE_NAMESPACE.into()),
            }]);

            let locator = make_locator(
                Some(FIXTURE_HOST),
                Some(FIXTURE_NAMESPACE),
                FIXTURE_IMAGE,
                Some(FIXTURE_TAG),
            );

            let path1 = loader
                .load_plugin(Id::raw("test"), locator.clone())
                .await
                .unwrap();

            assert!(path1.exists());

            let mtime1 = path1.metadata().unwrap().modified().unwrap();

            let path2 = loader.load_plugin(Id::raw("test"), locator).await.unwrap();

            assert_eq!(path1, path2);

            let mtime2 = path2.metadata().unwrap().modified().unwrap();

            assert_eq!(mtime1, mtime2);
        }

        // The primary motivation for this branch: an OCI plugin whose layer
        // is TOML/YAML/JSON used to silently re-pull on every load because
        // the cache lookup was hardcoded to `.wasm`. After `add_extensions`
        // is called, the probe must find a pre-existing non-WASM cache file
        // and skip the network round-trip entirely.
        //
        // We point the locator at a bogus host/image so any actual OCI pull
        // would error out at the network layer — `unwrap()` succeeding is
        // proof the registry was never contacted.
        #[tokio::test]
        async fn cached_non_wasm_layer_skips_oci_network() {
            let (_sandbox, mut loader) = create_loader_with_registries(vec![]);
            loader.add_extensions(vec!["toml".into()]);

            let locator = make_locator(
                Some("warpgate-test-nonexistent.invalid"),
                Some("nope"),
                "no_such_image",
                Some("v0.0.0-bogus"),
            );

            let hash = hash_sha256(locator.to_string());
            let toml_path = loader.create_cache_path(&Id::raw("test"), &hash, "toml", false);
            fs::create_dir_all(toml_path.parent().unwrap()).unwrap();
            fs::write_file(&toml_path, b"# fake toml plugin").unwrap();

            let result = loader.load_plugin(Id::raw("test"), locator).await.unwrap();

            assert_eq!(result, toml_path);
        }
    }
}
