use serde_json::{Value, json};
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::task::JoinSet;
use warpgate::host::{HostData, create_host_functions};
use warpgate::{
    Id, MAX_INSTANCES_CONFIG_KEY, PluginContainer, PluginLocator, PluginManifest, Wasm,
    create_http_client, find_debug_locator, inject_default_manifest_config,
};

fn create_container_for(
    sandbox: &Path,
    wasm_name: &str,
    max_instances: Option<&str>,
) -> PluginContainer {
    let id = Id::raw("moonstone");

    let wasm_file = find_debug_locator(wasm_name)
        .and_then(|locator| match locator {
            PluginLocator::File(file) => file.path.clone(),
            _ => None,
        })
        .expect("Test plugins not available. Run `just build-wasm` to build them!");

    let mut manifest = PluginManifest::new([Wasm::file(wasm_file)]);
    manifest.timeout_ms = None;

    if let Some(max) = max_instances {
        manifest
            .config
            .insert(MAX_INSTANCES_CONFIG_KEY.into(), max.into());
    }

    inject_default_manifest_config(&id, &sandbox.join("home"), &mut manifest).unwrap();

    let functions = create_host_functions(HostData {
        cache_dir: sandbox.join("cache"),
        http_client: Arc::new(create_http_client().unwrap()),
        virtual_paths: vec![],
        working_dir: sandbox.to_path_buf(),
    });

    PluginContainer::new(id, manifest, functions).unwrap()
}

fn create_sandboxed_container() -> (Sandbox, PluginContainer) {
    let sandbox = create_empty_sandbox();

    // Opt-in to parallel execution to exercise the instance pool
    let container = create_container_for(sandbox.path(), "proto_mocked_tool", Some("4"));

    (sandbox, container)
}

fn create_context() -> Value {
    json!({
        "temp_dir": "/temp",
        "tool_dir": "/tool",
        "working_dir": "/cwd",
    })
}

mod plugin_container {
    use super::*;

    #[tokio::test]
    async fn calls_a_function_and_returns_output() {
        let (_sandbox, container) = create_sandboxed_container();

        let output: Value = container
            .call_func_with("register_tool", json!({ "id": "moonstone" }))
            .await
            .unwrap();

        assert_eq!(output["name"], "moonstone");
    }

    #[tokio::test]
    async fn detects_existence_of_functions() {
        let (_sandbox, container) = create_sandboxed_container();

        assert!(container.has_func("register_tool").await);
        assert!(container.has_func("load_versions").await);
        assert!(!container.has_func("does_not_exist").await);
    }

    #[tokio::test]
    async fn calls_same_function_repeatedly_across_instances() {
        let (_sandbox, container) = create_sandboxed_container();

        // This function initializes guest state (a tracing subscriber),
        // so repeated calls verify that instances can be safely reused
        for _ in 0..3 {
            let output: Value = container
                .call_func_with("register_tool", json!({ "id": "moonstone" }))
                .await
                .unwrap();

            assert_eq!(output["name"], "moonstone");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn calls_functions_concurrently() {
        let (_sandbox, container) = create_sandboxed_container();
        let container = Arc::new(container);
        let mut set = JoinSet::new();

        for _ in 0..12 {
            let container = Arc::clone(&container);

            set.spawn(async move {
                let output: Value = container
                    .call_func_with(
                        "detect_version_files",
                        json!({ "context": create_context() }),
                    )
                    .await
                    .unwrap();

                assert_eq!(output["files"][0], ".moonstonerc");
            });
        }

        set.join_all().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn only_calls_cached_function_once_when_called_concurrently() {
        let (_sandbox, container) = create_sandboxed_container();
        let container = Arc::new(container);
        let calls = Arc::new(AtomicUsize::new(0));

        container.set_on_call(Arc::new({
            let calls = Arc::clone(&calls);

            move |_func, input, _output| {
                if input.is_some() {
                    calls.fetch_add(1, Ordering::SeqCst);
                }
            }
        }));

        let mut set = JoinSet::new();

        for _ in 0..12 {
            let container = Arc::clone(&container);

            set.spawn(async move {
                let output: Value = container
                    .cache_func_with(
                        "detect_version_files",
                        json!({ "context": create_context() }),
                    )
                    .await
                    .unwrap();

                assert_eq!(output["files"][0], ".moonstonerc");
            });
        }

        set.join_all().await;

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn caches_function_output_per_input() {
        let (_sandbox, container) = create_sandboxed_container();
        let calls = Arc::new(AtomicUsize::new(0));

        container.set_on_call(Arc::new({
            let calls = Arc::clone(&calls);

            move |_func, input, _output| {
                if input.is_some() {
                    calls.fetch_add(1, Ordering::SeqCst);
                }
            }
        }));

        let input_one = json!({ "context": create_context() });
        let input_two = json!({
            "context": {
                "temp_dir": "/temp",
                "tool_dir": "/tool",
                "working_dir": "/elsewhere",
            }
        });

        for _ in 0..3 {
            let _: Value = container
                .cache_func_with("detect_version_files", input_one.clone())
                .await
                .unwrap();
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let _: Value = container
            .cache_func_with("detect_version_files", input_two)
            .await
            .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn persists_var_state_across_calls_by_default() {
        let sandbox = create_empty_sandbox();

        // No opt-in, so a single instance is used for all calls
        let container = create_container_for(sandbox.path(), "proto_api_usage", None);

        container
            .call("set_var_state", "pooled-state")
            .await
            .unwrap();

        let value = container.call("get_var_state", "").await.unwrap();

        assert_eq!(String::from_utf8_lossy(&value), "pooled-state");

        // A clean error (not a trap) must not discard the
        // instance, otherwise its variable state is lost
        container.call("does_not_exist", "").await.unwrap_err();

        let value = container.call("get_var_state", "").await.unwrap();

        assert_eq!(String::from_utf8_lossy(&value), "pooled-state");
    }

    #[tokio::test]
    async fn discards_instance_and_state_after_a_trap() {
        let sandbox = create_empty_sandbox();
        let container = create_container_for(sandbox.path(), "proto_api_usage", None);

        container
            .call("set_var_state", "pooled-state")
            .await
            .unwrap();
        container.call("trigger_trap", "").await.unwrap_err();

        // The instance was discarded, so a new instance
        // is created without the previous variable state
        let value = container.call("get_var_state", "").await.unwrap();

        assert_eq!(String::from_utf8_lossy(&value), "");
    }

    #[tokio::test]
    async fn recovers_after_failed_calls() {
        let (_sandbox, container) = create_sandboxed_container();

        // Clean errors are part of normal plugin operation,
        // so trigger a few and ensure new calls still succeed
        for _ in 0..3 {
            let result = container
                .call_func_with::<_, _, Value>(
                    "parse_version_file",
                    json!({
                        "content": "invalid version!",
                        "context": create_context(),
                        "file": ".moonstonerc",
                        "path": "/cwd/.moonstonerc",
                    }),
                )
                .await;

            assert!(result.is_err());
        }

        let output: Value = container
            .call_func_with(
                "detect_version_files",
                json!({ "context": create_context() }),
            )
            .await
            .unwrap();

        assert_eq!(output["files"][0], ".moonstonerc");
    }
}
