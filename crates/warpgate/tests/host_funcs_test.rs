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

    PluginContainer::new(
        id,
        manifest,
        create_host_functions(HostData {
            cache_dir: sandbox_path.join("cache"),
            http_client: Arc::new(create_http_client().unwrap()),
            virtual_paths: vec![(sandbox_path.to_path_buf(), PathBuf::from("/sandbox"))],
            working_dir: sandbox_path.to_path_buf(),
        }),
    )
    .unwrap()
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

        let result = container
            .call_func_with::<_, _, SendRequestOutput>(
                "testing_send_request",
                SendRequestInput::new(url),
            )
            .await;

        assert!(result.is_err());
    }
}
