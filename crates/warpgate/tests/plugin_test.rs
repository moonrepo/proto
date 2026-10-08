use starbase_sandbox::create_empty_sandbox;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::task::JoinSet;
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

fn create_container(sandbox_path: &Path) -> PluginContainer {
    let id = Id::raw("test");
    let mut manifest = PluginManifest::new([Wasm::file(find_api_usage_wasm())]);

    inject_default_manifest_config(&id, &sandbox_path.join("home"), &mut manifest).unwrap();

    PluginContainer::new(
        id,
        manifest,
        create_host_functions(HostData {
            cache_dir: sandbox_path.join("cache"),
            http_client: Arc::new(create_http_client().unwrap()),
            virtual_paths: vec![],
            working_dir: sandbox_path.to_path_buf(),
        }),
    )
    .unwrap()
}

// Count the number of guest function calls, which excludes cache hits
fn count_calls(container: &PluginContainer) -> Arc<AtomicUsize> {
    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&count);

    container.set_on_call(Arc::new(move |_, input, _| {
        // Called before (with input) and after (with output)
        if input.is_some() {
            count_clone.fetch_add(1, Ordering::SeqCst);
        }
    }));

    count
}

mod has_func {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_true_if_exists() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());

        assert!(container.has_func("testing_echo").await);
        // Cached
        assert!(container.has_func("testing_echo").await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_false_if_missing() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());

        assert!(!container.has_func("unknown_func").await);
        // Cached
        assert!(!container.has_func("unknown_func").await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_same_result_when_concurrent() {
        let sandbox = create_empty_sandbox();
        let container = Arc::new(create_container(sandbox.path()));
        let mut set = JoinSet::new();

        for i in 0..100 {
            let container = Arc::clone(&container);

            set.spawn(async move {
                if i % 2 == 0 {
                    container.has_func("testing_echo").await
                } else {
                    !container.has_func("unknown_func").await
                }
            });
        }

        while let Some(result) = set.join_next().await {
            assert!(result.unwrap());
        }
    }
}

mod cache_func_with {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_cached_output_for_same_input() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());
        let calls = count_calls(&container);

        let a: String = container
            .cache_func_with("testing_echo", "a")
            .await
            .unwrap();
        let b: String = container
            .cache_func_with("testing_echo", "a")
            .await
            .unwrap();

        assert_eq!(a, "a");
        assert_eq!(b, "a");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn calls_again_for_different_input() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());
        let calls = count_calls(&container);

        for input in ["a", "b", "a", "b"] {
            let output: String = container
                .cache_func_with("testing_echo", input)
                .await
                .unwrap();

            assert_eq!(output, input);
        }

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_cache_errors() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());
        let calls = count_calls(&container);

        // Function expects a string
        for _ in 0..2 {
            assert!(
                container
                    .cache_func_with::<_, _, String>("testing_echo", 123)
                    .await
                    .is_err()
            );
        }

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_cache_when_disabled() {
        let sandbox = create_empty_sandbox();

        unsafe { env::set_var("WARPGATE_NO_FUNC_CACHE", "true") };

        let container = create_container(sandbox.path());

        unsafe { env::remove_var("WARPGATE_NO_FUNC_CACHE") };

        let calls = count_calls(&container);

        for _ in 0..2 {
            let output: String = container
                .cache_func_with("testing_echo", "a")
                .await
                .unwrap();

            assert_eq!(output, "a");
        }

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_same_result_when_concurrent() {
        let sandbox = create_empty_sandbox();
        let container = Arc::new(create_container(sandbox.path()));
        let calls = count_calls(&container);
        let mut set = JoinSet::new();

        for i in 0..100 {
            let container = Arc::clone(&container);

            set.spawn(async move {
                let input = format!("input-{}", i % 5);
                let output: String = container
                    .cache_func_with("testing_echo", &input)
                    .await
                    .unwrap();

                assert_eq!(output, input);
            });
        }

        while let Some(result) = set.join_next().await {
            result.unwrap();
        }

        // Concurrent misses for the same input may each call the function
        let count = calls.load(Ordering::SeqCst);

        assert!((5..=100).contains(&count), "{count}");

        // But afterwards, every input is cached
        for i in 0..5 {
            let _: String = container
                .cache_func_with("testing_echo", format!("input-{i}"))
                .await
                .unwrap();
        }

        assert_eq!(calls.load(Ordering::SeqCst), count);
    }
}
