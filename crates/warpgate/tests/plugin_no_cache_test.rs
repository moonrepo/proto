// This test sets `WARPGATE_NO_FUNC_CACHE`, which is read when a container is
// created, so it runs in its own binary (process). Otherwise it would disable
// caching for containers created by other tests running in parallel.

use starbase_sandbox::create_empty_sandbox;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use warpgate::host::{HostData, create_host_functions};
use warpgate::{
    Id, PluginContainer, PluginManifest, Wasm, create_http_client, inject_default_manifest_config,
    test_utils,
};

fn find_api_usage_wasm() -> PathBuf {
    let plugins_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins");

    test_utils::find_target_dir(plugins_dir)
        .map(|dir| dir.join("proto_api_usage.wasm"))
        .filter(|file| file.exists())
        .expect("proto_api_usage.wasm does not exist. Please build it with `just build-wasm` before running tests!")
}

#[tokio::test(flavor = "multi_thread")]
async fn doesnt_cache_when_disabled() {
    let sandbox = create_empty_sandbox();
    let id = Id::raw("test");
    let mut manifest = PluginManifest::new([Wasm::file(find_api_usage_wasm())]);

    inject_default_manifest_config(&id, &sandbox.path().join("home"), &mut manifest).unwrap();

    // Safe, as this is the only test in this binary
    unsafe { env::set_var("WARPGATE_NO_FUNC_CACHE", "true") };

    let container = PluginContainer::new(
        id,
        manifest,
        create_host_functions(HostData {
            cache_dir: sandbox.path().join("cache"),
            foreign_platform: None,
            http_client: Arc::new(create_http_client().unwrap()),
            virtual_paths: vec![],
            working_dir: sandbox.path().to_path_buf(),
        }),
    )
    .unwrap();

    // Count guest function calls, which excludes cache hits
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_clone = Arc::clone(&calls);

    container.set_on_call(Arc::new(move |_, input, _| {
        // Called before (with input) and after (with output)
        if input.is_some() {
            calls_clone.fetch_add(1, Ordering::SeqCst);
        }
    }));

    for _ in 0..2 {
        let output: String = container
            .cache_func_with("testing_echo", "a")
            .await
            .unwrap();

        assert_eq!(output, "a");
    }

    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
