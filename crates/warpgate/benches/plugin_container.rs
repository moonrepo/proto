use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::task::JoinSet;
use warpgate::host::{HostData, create_host_functions};
use warpgate::{
    Id, PluginContainer, PluginManifest, Wasm, create_http_client, inject_default_manifest_config,
    test_utils,
};

// Number of calls that each concurrent task makes
const CALLS_PER_TASK: usize = 10;

// Number of tasks that call the same container concurrently
const TASKS: [usize; 3] = [1, 10, 100];

fn find_api_usage_wasm() -> PathBuf {
    test_utils::find_wasm_file_with_name("proto_api_usage")
        .or_else(|| {
            let plugins_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins");

            test_utils::find_target_dir(plugins_dir)
                .map(|dir| dir.join("proto_api_usage.wasm"))
                .filter(|file| file.exists())
        })
        .expect("proto_api_usage.wasm does not exist. Please build it with `just build-wasm` before running benchmarks!")
}

fn create_container(sandbox: &Sandbox) -> Arc<PluginContainer> {
    let id = Id::raw("bench");
    let mut manifest = PluginManifest::new([Wasm::file(find_api_usage_wasm())]);

    inject_default_manifest_config(&id, &sandbox.path().join("home"), &mut manifest).unwrap();

    Arc::new(
        PluginContainer::new(
            id,
            manifest,
            create_host_functions(HostData {
                cache_dir: sandbox.path().join("cache"),
                http_client: Arc::new(create_http_client().unwrap()),
                virtual_paths: vec![],
                working_dir: sandbox.path().to_path_buf(),
            }),
        )
        .unwrap(),
    )
}

// Spawn tasks that each run the operation `CALLS_PER_TASK` times against
// the same container, and wait for all of them to complete
async fn run_concurrent<F, Fut>(container: &Arc<PluginContainer>, tasks: usize, op: F)
where
    F: Fn(Arc<PluginContainer>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut set = JoinSet::new();

    for _ in 0..tasks {
        set.spawn(op(Arc::clone(container)));
    }

    while let Some(result) = set.join_next().await {
        result.unwrap();
    }
}

fn bench_container(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let sandbox = create_empty_sandbox();
    let container = create_container(&sandbox);

    // Warm the caches, so that every benchmarked call is a hit
    runtime.block_on(async {
        container.has_func("testing_echo").await;
        container
            .cache_func_with::<_, _, String>("testing_echo", "input")
            .await
            .unwrap();
    });

    let mut group = c.benchmark_group("PluginContainer");

    for tasks in TASKS {
        group.throughput(Throughput::Elements((tasks * CALLS_PER_TASK) as u64));

        group.bench_with_input(BenchmarkId::new("has_func", tasks), &tasks, |b, &tasks| {
            b.to_async(&runtime).iter(|| {
                run_concurrent(&container, tasks, |container| async move {
                    for _ in 0..CALLS_PER_TASK {
                        black_box(container.has_func("testing_echo").await);
                    }
                })
            })
        });

        group.bench_with_input(
            BenchmarkId::new("cache_func_with", tasks),
            &tasks,
            |b, &tasks| {
                b.to_async(&runtime).iter(|| {
                    run_concurrent(&container, tasks, |container| async move {
                        for _ in 0..CALLS_PER_TASK {
                            let output: String = container
                                .cache_func_with("testing_echo", "input")
                                .await
                                .unwrap();

                            black_box(output);
                        }
                    })
                })
            },
        );

        // Check for a function before calling it, like consumers commonly do
        group.bench_with_input(BenchmarkId::new("mixed", tasks), &tasks, |b, &tasks| {
            b.to_async(&runtime).iter(|| {
                run_concurrent(&container, tasks, |container| async move {
                    for _ in 0..CALLS_PER_TASK {
                        if container.has_func("testing_echo").await {
                            let output: String = container
                                .cache_func_with("testing_echo", "input")
                                .await
                                .unwrap();

                            black_box(output);
                        }
                    }
                })
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_container);
criterion_main!(benches);
