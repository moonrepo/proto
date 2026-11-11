use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use rustc_hash::FxHashMap;
use starbase_sandbox::create_empty_sandbox;
use starbase_utils::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use warpgate::api::{DownloadFileInput, DownloadFileOutput, SendRequestInput, SendRequestOutput};
use warpgate::host::{HostData, create_host_functions};
use warpgate::{
    Id, PluginContainer, PluginManifest, VirtualPath, Wasm, create_http_client,
    inject_default_manifest_config, test_utils,
};

// Serve a static response from a local HTTP server, and return the URL to request.
fn spawn_http_server(status: &'static str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                break;
            };

            let mut request = [0; 1024];
            let _ = stream.read(&mut request);
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len(),
            );
        }
    });

    format!("http://{addr}/archive.txt")
}

// Serve a response that echoes the received request head (request line and
// headers) back as the response body, and return the URL to request.
fn spawn_echo_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                break;
            };

            let mut request = Vec::new();
            let mut buffer = [0u8; 1024];

            while let Ok(bytes) = stream.read(&mut buffer) {
                request.extend_from_slice(&buffer[..bytes]);

                if bytes == 0 || request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }

            let body = String::from_utf8_lossy(&request).into_owned();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len(),
            );
        }
    });

    format!("http://{addr}/request")
}

fn find_api_usage_wasm() -> PathBuf {
    let plugins_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins");

    test_utils::find_target_dir(plugins_dir)
        .map(|dir| dir.join("proto_api_usage.wasm"))
        .filter(|file| file.exists())
        .expect("proto_api_usage.wasm does not exist. Please build it with `just build-wasm` before running tests!")
}

fn create_container(sandbox_path: &std::path::Path) -> PluginContainer {
    create_container_for_platform(sandbox_path, None)
}

fn create_container_for_platform(
    sandbox_path: &std::path::Path,
    foreign_platform: Option<&str>,
) -> PluginContainer {
    let id = Id::raw("test");

    let mut manifest = PluginManifest::new([Wasm::file(find_api_usage_wasm())]);
    manifest = manifest.with_allowed_paths(
        [(
            sandbox_path.to_string_lossy().to_string(),
            PathBuf::from("/sandbox"),
        )]
        .into_iter(),
    );

    inject_default_manifest_config(&id, &sandbox_path.join("home"), &mut manifest).unwrap();

    let functions = create_test_host_functions(HostData {
        cache_dir: sandbox_path.join("cache"),
        foreign_platform: foreign_platform.map(|platform| platform.to_owned()),
        http_client: Arc::new(create_http_client().unwrap()),
        virtual_paths: vec![(sandbox_path.to_path_buf(), PathBuf::from("/sandbox"))],
        working_dir: sandbox_path.to_path_buf(),
    });

    PluginContainer::new(id, manifest, functions).unwrap()
}

fn create_test_host_functions(data: HostData) -> Vec<Function> {
    let mut functions = create_host_functions(data);
    functions.push(Function::new(
        "load_tool_versions",
        [ValType::I64],
        [ValType::I64],
        UserData::new(()),
        unused_proto_load_tool_versions,
    ));
    functions
}

fn unused_proto_load_tool_versions(
    plugin: &mut CurrentPlugin,
    _inputs: &[Val],
    outputs: &mut [Val],
    _user_data: UserData<()>,
) -> Result<(), Error> {
    plugin.memory_set_val(&mut outputs[0], r#"{"versions":[]}"#)?;

    Ok(())
}

mod path_ext {
    use super::*;
    use warpgate::api::{HostEnvironment, HostOS};

    fn create_windows_container(sandbox_path: &std::path::Path) -> PluginContainer {
        let id = Id::raw("test");
        let mut manifest = PluginManifest::new([Wasm::file(find_api_usage_wasm())]);

        manifest.config.insert(
            "virtual_paths".into(),
            r#"[["C:\\Users\\me", "/userhome"]]"#.into(),
        );

        inject_default_manifest_config(&id, &sandbox_path.join("home"), &mut manifest).unwrap();

        let mut env: HostEnvironment =
            serde_json::from_str(&manifest.config["host_environment"]).unwrap();
        env.os = HostOS::Windows;

        manifest.config.insert(
            "host_environment".into(),
            serde_json::to_string(&env).unwrap(),
        );

        PluginContainer::new(
            id,
            manifest,
            create_test_host_functions(HostData {
                cache_dir: sandbox_path.join("cache"),
                foreign_platform: None,
                http_client: Arc::new(create_http_client().unwrap()),
                virtual_paths: vec![],
                working_dir: sandbox_path.to_path_buf(),
            }),
        )
        .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn real_paths_only_use_forward_slashes_on_windows() {
        let sandbox = create_empty_sandbox();
        let container = create_windows_container(sandbox.path());

        let output: Option<String> = container
            .call_func_with("testing_to_real_path", "/userhome/.proto/tools")
            .await
            .unwrap();

        assert_eq!(output.unwrap(), "C:/Users/me/.proto/tools");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn creates_virtual_paths_from_windows_paths() {
        let sandbox = create_empty_sandbox();
        let container = create_windows_container(sandbox.path());

        let output: String = container
            .call_func_with("testing_create_virtual_path", r"C:\Users\me\.cargo")
            .await
            .unwrap();

        assert_eq!(output, "/userhome/.cargo");
    }
}

mod download_file {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_url_to_host_file() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());
        let url = spawn_http_server("200 OK", "archive contents");

        let output: DownloadFileOutput = container
            .call_func_with(
                "testing_download_file",
                DownloadFileInput::new(url, "/sandbox/dl/archive.txt"),
            )
            .await
            .unwrap();

        assert_eq!(output.file, VirtualPath::new("/sandbox/dl/archive.txt"));
        assert_eq!(output.size, 16);

        let dest_file = sandbox.path().join("dl/archive.txt");

        assert!(dest_file.exists());
        assert_eq!(fs::read_file(dest_file).unwrap(), "archive contents");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn errors_when_url_is_not_found() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());
        let url = spawn_http_server("404 Not Found", "");

        let result = container
            .call_func_with::<_, _, DownloadFileOutput>(
                "testing_download_file",
                DownloadFileInput::new(url, "/sandbox/dl/archive.txt"),
            )
            .await;

        assert!(result.is_err());
        assert!(!sandbox.path().join("dl/archive.txt").exists());
    }
}

mod env_vars {
    use super::*;
    use std::env;

    #[tokio::test(flavor = "multi_thread")]
    async fn gets_multiple_vars() {
        unsafe {
            env::set_var("WARPGATE_TEST_GET_ENV_VARS_A", "a");
            env::set_var("WARPGATE_TEST_GET_ENV_VARS_B", "b");
        };

        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());

        let vars: FxHashMap<String, String> = container
            .call_func_with(
                "testing_get_env_vars",
                [
                    "WARPGATE_TEST_GET_ENV_VARS_A",
                    "WARPGATE_TEST_GET_ENV_VARS_B",
                ],
            )
            .await
            .unwrap();

        assert_eq!(
            vars,
            FxHashMap::from_iter([
                ("WARPGATE_TEST_GET_ENV_VARS_A".into(), "a".into()),
                ("WARPGATE_TEST_GET_ENV_VARS_B".into(), "b".into()),
            ])
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn omits_unset_vars_but_keeps_empty_vars() {
        unsafe {
            env::set_var("WARPGATE_TEST_GET_ENV_VARS_EMPTY", "");
            env::remove_var("WARPGATE_TEST_GET_ENV_VARS_UNSET");
        };

        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());

        let vars: FxHashMap<String, String> = container
            .call_func_with(
                "testing_get_env_vars",
                [
                    "WARPGATE_TEST_GET_ENV_VARS_EMPTY",
                    "WARPGATE_TEST_GET_ENV_VARS_UNSET",
                ],
            )
            .await
            .unwrap();

        assert_eq!(
            vars,
            FxHashMap::from_iter([("WARPGATE_TEST_GET_ENV_VARS_EMPTY".into(), "".into())])
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_multiple_vars() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());

        container
            .call_func_without_output(
                "testing_set_env_vars",
                FxHashMap::from_iter([
                    ("WARPGATE_TEST_SET_ENV_VARS_A", "a"),
                    ("WARPGATE_TEST_SET_ENV_VARS_B", "b"),
                ]),
            )
            .await
            .unwrap();

        assert_eq!(env::var("WARPGATE_TEST_SET_ENV_VARS_A").unwrap(), "a");
        assert_eq!(env::var("WARPGATE_TEST_SET_ENV_VARS_B").unwrap(), "b");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn appends_real_paths_when_setting_path() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());

        container
            .call_func_without_output(
                "testing_set_env_vars",
                FxHashMap::from_iter([("PATH", "/sandbox/env-vars/a:/sandbox/env-vars/b")]),
            )
            .await
            .unwrap();

        let paths = env::split_paths(&env::var_os("PATH").unwrap()).collect::<Vec<_>>();

        assert_eq!(
            paths[paths.len() - 2..],
            [
                sandbox.path().join("env-vars/a"),
                sandbox.path().join("env-vars/b")
            ]
        );
    }
}

mod send_request {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn sends_a_get_request_by_default() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());
        let url = spawn_echo_server();

        let output: SendRequestOutput = container
            .call_func_with("testing_send_request", SendRequestInput::new(url))
            .await
            .unwrap();

        assert_eq!(output.status, 200);
        assert!(
            output
                .text()
                .unwrap()
                .starts_with("GET /request HTTP/1.1\r\n")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sends_a_post_request() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());
        let url = spawn_echo_server();

        let output: SendRequestOutput = container
            .call_func_with("testing_send_request", SendRequestInput::post(url))
            .await
            .unwrap();

        assert_eq!(output.status, 200);
        assert!(
            output
                .text()
                .unwrap()
                .starts_with("POST /request HTTP/1.1\r\n")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn expands_env_vars_in_request_headers() {
        unsafe { std::env::set_var("WARPGATE_TEST_SEND_REQUEST_TOKEN", "wg-secret-value") };

        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());
        let url = spawn_echo_server();

        let output: SendRequestOutput = container
            .call_func_with(
                "testing_send_request",
                SendRequestInput::new(url).headers(FxHashMap::from_iter([(
                    "Authorization".into(),
                    "Bearer ${WARPGATE_TEST_SEND_REQUEST_TOKEN}".into(),
                )])),
            )
            .await
            .unwrap();

        let request = output.text().unwrap().to_lowercase();

        assert!(!request.contains("${"));
        assert!(request.contains("authorization: bearer wg-secret-value"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn errors_when_response_is_not_ok() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());
        let url = spawn_http_server("404 Not Found", "");

        let error = container
            .call_func_with::<_, _, SendRequestOutput>(
                "testing_send_request",
                SendRequestInput::new(url),
            )
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("Failed to request"), "{error}");
        assert!(error.contains("(404)"), "{error}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn errors_when_response_is_a_server_error() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());
        let url = spawn_http_server("500 Internal Server Error", "oops");

        let error = container
            .call_func_with::<_, _, SendRequestOutput>(
                "testing_send_request",
                SendRequestInput::new(url),
            )
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("(500)"), "{error}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_not_ok_response_when_not_erroring_on_failure() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());
        let url = spawn_http_server("404 Not Found", "missing");

        let output: SendRequestOutput = container
            .call_func_with(
                "testing_send_request",
                SendRequestInput::new(url).error_on_failure(false),
            )
            .await
            .unwrap();

        assert_eq!(output.status, 404);
        assert_eq!(output.text().unwrap(), "missing");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_empty_body_when_not_erroring_on_failure() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());
        let url = spawn_http_server("404 Not Found", "");

        let output: SendRequestOutput = container
            .call_func_with(
                "testing_send_request",
                SendRequestInput::new(url).error_on_failure(false),
            )
            .await
            .unwrap();

        assert_eq!(output.status, 404);
        assert!(output.body.is_empty());
    }
}

mod exec_command {
    use super::*;
    use warpgate::api::{ExecCommandInput, ExecCommandOutput};

    #[tokio::test(flavor = "multi_thread")]
    async fn executes_command_on_host() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());

        let output: ExecCommandOutput = container
            .call_func_with(
                "testing_exec_command",
                ExecCommandInput::pipe("git", ["--version"]),
            )
            .await
            .unwrap();

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("git version"), "{}", output.stdout);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_nonzero_exit_code_by_default() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());

        let output: ExecCommandOutput = container
            .call_func_with(
                "testing_exec_command",
                ExecCommandInput::pipe("git", ["unknown-subcommand"]),
            )
            .await
            .unwrap();

        assert_eq!(output.exit_code, 1);
        assert!(
            output.stderr.contains("is not a git command"),
            "{}",
            output.stderr
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn errors_with_stderr_on_nonzero_exit_code() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());

        let error = container
            .call_func_with::<_, _, ExecCommandOutput>(
                "testing_exec_command",
                ExecCommandInput::pipe("git", ["unknown-subcommand"]).error_on_nonzero(true),
            )
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("git unknown-subcommand"), "{error}");
        assert!(error.contains("failed with a 1 exit code"), "{error}");
        assert!(error.contains("is not a git command"), "{error}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_error_on_zero_exit_code() {
        let sandbox = create_empty_sandbox();
        let container = create_container(sandbox.path());

        let output: ExecCommandOutput = container
            .call_func_with(
                "testing_exec_command",
                ExecCommandInput::pipe("git", ["--version"]).error_on_nonzero(true),
            )
            .await
            .unwrap();

        assert_eq!(output.exit_code, 0);
    }

    // Only a warning is logged, as plugins decide with `HostEnvironment.foreign`
    #[tokio::test(flavor = "multi_thread")]
    async fn executes_command_when_targeting_a_foreign_platform() {
        let sandbox = create_empty_sandbox();
        let container = create_container_for_platform(sandbox.path(), Some("x64-linux-musl"));

        let output: ExecCommandOutput = container
            .call_func_with(
                "testing_exec_command",
                ExecCommandInput::pipe("git", ["--version"]),
            )
            .await
            .unwrap();

        assert_eq!(output.exit_code, 0);
    }
}
