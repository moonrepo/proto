use rustc_hash::FxHashMap;
use starbase_utils::net::Downloader;
use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use warpgate::{HttpOptions, create_http_client, create_http_client_with_options};

// Env var names are unique per test since tests run in parallel threads
// within the same process.
fn set_env(key: &str, value: &str) {
    unsafe { env::set_var(key, value) };
}

// Accept a single connection, capture the raw request head, and reply
// with an empty 200 so the client resolves without retrying.
fn spawn_server(listener: TcpListener) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];

        loop {
            let bytes = stream.read(&mut buffer).unwrap();
            request.extend_from_slice(&buffer[..bytes]);

            if bytes == 0 || request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        stream
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
            .unwrap();

        String::from_utf8_lossy(&request).into_owned()
    })
}

// Download a file with the provided options, and return the raw request head.
async fn capture_request(options: HttpOptions) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = spawn_server(listener);

    let downloader = create_http_client_with_options(&options)
        .unwrap()
        .create_downloader();

    downloader
        .download(format!("http://{addr}/file.txt").parse().unwrap())
        .await
        .unwrap();

    server.join().unwrap()
}

mod expand_env_vars {
    use super::*;

    #[test]
    fn expands_set_var() {
        set_env("WARPGATE_TEST_EXPAND_SINGLE", "abc123");

        let client = create_http_client().unwrap();

        assert_eq!(
            client.expand_env_vars("Bearer ${WARPGATE_TEST_EXPAND_SINGLE}"),
            "Bearer abc123"
        );
    }

    #[test]
    fn expands_var_set_to_empty_string() {
        set_env("WARPGATE_TEST_EXPAND_EMPTY", "");

        let client = create_http_client().unwrap();

        assert_eq!(
            client.expand_env_vars("Bearer ${WARPGATE_TEST_EXPAND_EMPTY}"),
            "Bearer "
        );
    }

    #[test]
    fn keeps_placeholder_when_var_unset() {
        let client = create_http_client().unwrap();

        assert_eq!(
            client.expand_env_vars("Bearer ${WARPGATE_TEST_EXPAND_UNSET}"),
            "Bearer ${WARPGATE_TEST_EXPAND_UNSET}"
        );
    }

    #[test]
    fn expands_multiple_vars_and_keeps_unset_ones() {
        set_env("WARPGATE_TEST_EXPAND_MIXED_SET", "abc");

        let client = create_http_client().unwrap();

        assert_eq!(
            client.expand_env_vars(
                "${WARPGATE_TEST_EXPAND_MIXED_SET}:${WARPGATE_TEST_EXPAND_MIXED_UNSET}"
            ),
            "abc:${WARPGATE_TEST_EXPAND_MIXED_UNSET}"
        );
    }

    #[test]
    fn expands_repeated_occurrences_of_same_var() {
        set_env("WARPGATE_TEST_EXPAND_REPEAT", "xyz");

        let client = create_http_client().unwrap();

        assert_eq!(
            client.expand_env_vars(
                "${WARPGATE_TEST_EXPAND_REPEAT} and ${WARPGATE_TEST_EXPAND_REPEAT}"
            ),
            "xyz and xyz"
        );
    }

    #[test]
    fn returns_value_without_placeholders_unchanged() {
        let client = create_http_client().unwrap();

        assert_eq!(client.expand_env_vars(""), "");
        assert_eq!(
            client.expand_env_vars("application/vnd.github+json"),
            "application/vnd.github+json"
        );
    }

    // Only the `${VAR}` syntax is supported, where VAR is uppercase
    // alphanumeric or underscore.
    #[test]
    fn ignores_unsupported_syntax_even_when_var_is_set() {
        set_env("WARPGATE_TEST_EXPAND_BRACELESS", "abc");
        set_env("warpgate_test_expand_lower", "abc");

        let client = create_http_client().unwrap();

        for value in [
            "$WARPGATE_TEST_EXPAND_BRACELESS",
            "${warpgate_test_expand_lower}",
            "${WARPGATE-TEST-EXPAND-DASHES}",
            "${}",
        ] {
            assert_eq!(client.expand_env_vars(value), value);
        }
    }
}

mod downloader_headers {
    use super::*;

    #[tokio::test]
    async fn expands_env_vars_in_request_headers() {
        set_env("WARPGATE_TEST_DOWNLOAD_TOKEN", "wg-secret-value");

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = spawn_server(listener);

        let downloader = create_http_client()
            .unwrap()
            .create_downloader_with_headers(FxHashMap::from_iter([(
                "Authorization".into(),
                "Bearer ${WARPGATE_TEST_DOWNLOAD_TOKEN}".into(),
            )]));

        let response = downloader
            .download(format!("http://{addr}/file.txt").parse().unwrap())
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), 200);

        let request = server.join().unwrap();

        assert!(!request.contains("${"));
        assert!(
            request
                .to_lowercase()
                .contains("authorization: bearer wg-secret-value")
        );
    }
}

mod user_agent {
    use super::*;

    #[tokio::test]
    async fn keeps_client_default_when_not_configured() {
        let request = capture_request(HttpOptions::default()).await;

        assert!(request.contains(&format!(
            "user-agent: warpgate@{}\r\n",
            env!("CARGO_PKG_VERSION")
        )));
    }

    #[tokio::test]
    async fn sends_configured_value() {
        let request = capture_request(HttpOptions {
            user_agent: Some("acme-corp/1.2.3 (+https://acme.corp)".into()),
            ..Default::default()
        })
        .await;

        assert!(request.contains("user-agent: acme-corp/1.2.3 (+https://acme.corp)\r\n"));
        assert!(!request.contains("warpgate@"));
    }

    #[test]
    fn errors_when_empty() {
        let error = create_http_client_with_options(&HttpOptions {
            user_agent: Some("".into()),
            ..Default::default()
        })
        .unwrap_err();

        assert!(error.to_string().starts_with("Invalid user agent"));
    }

    #[test]
    fn errors_when_not_a_valid_header_value() {
        for value in [
            "proto\nX-Injected: yes",
            "proto\rX-Injected: yes",
            "proto\u{0}",
        ] {
            let error = create_http_client_with_options(&HttpOptions {
                user_agent: Some(value.into()),
                ..Default::default()
            })
            .unwrap_err();

            assert!(error.to_string().starts_with("Invalid user agent"));
        }
    }
}
