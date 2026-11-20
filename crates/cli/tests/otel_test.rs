use proto_core::test_utils::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

type Requests = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

// A minimal OTLP/HTTP collector, that records the path and body of every
// export request. The exporters shut down (and flush) synchronously before
// proto exits, so every request has been recorded once the command finishes.
struct Collector {
    endpoint: String,
    requests: Requests,
}

impl Collector {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let requests = Requests::default();
        let requests_clone = Arc::clone(&requests);

        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let requests = Arc::clone(&requests_clone);

                thread::spawn(move || handle_connection(stream, requests));
            }
        });

        Self { endpoint, requests }
    }

    fn bodies(&self, path: &str) -> Vec<Vec<u8>> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(request_path, _)| request_path == path)
            .map(|(_, body)| body.clone())
            .collect()
    }

    // Span names are encoded as field 5 of the `Span` protobuf message
    fn has_span(&self, name: &str) -> bool {
        let mut needle = vec![0x2a, name.len() as u8];
        needle.extend(name.as_bytes());

        self.bodies("/v1/traces")
            .iter()
            .any(|body| body.windows(needle.len()).any(|window| window == needle))
    }

    fn has_metric(&self, name: &str) -> bool {
        self.bodies("/v1/metrics").iter().any(|body| {
            body.windows(name.len())
                .any(|window| window == name.as_bytes())
        })
    }
}

fn handle_connection(stream: TcpStream, requests: Requests) {
    let mut writer = stream.try_clone().unwrap();
    let mut reader = BufReader::new(stream);

    // Exporters may reuse the connection for multiple requests
    loop {
        let mut request_line = String::new();

        if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
            return;
        }

        let path = request_line
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
            .to_owned();
        let mut content_length = 0;

        loop {
            let mut header = String::new();

            if reader.read_line(&mut header).unwrap_or(0) == 0 {
                return;
            }

            let header = header.trim_end();

            if header.is_empty() {
                break;
            }

            if let Some((name, value)) = header.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }

        let mut body = vec![0; content_length];

        if reader.read_exact(&mut body).is_err() {
            return;
        }

        requests.lock().unwrap().push((path, body));

        if writer
            .write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: application/x-protobuf\r\ncontent-length: 0\r\n\r\n",
            )
            .is_err()
        {
            return;
        }
    }
}

fn export_to(cmd: &mut starbase_sandbox::assert_cmd::Command, collector: &Collector) {
    cmd.arg("--otel")
        .env("OTEL_EXPORTER_OTLP_ENDPOINT", &collector.endpoint)
        .env("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");

    // Don't let the environment running the tests redirect or disable exports
    for key in [
        "OTEL_SDK_DISABLED",
        "OTEL_TRACES_EXPORTER",
        "OTEL_METRICS_EXPORTER",
        "OTEL_LOGS_EXPORTER",
        "OTEL_EXPORTER_OTLP_COMPRESSION",
        "OTEL_EXPORTER_OTLP_HEADERS",
        "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
        "OTEL_EXPORTER_OTLP_TRACES_PROTOCOL",
        "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
        "OTEL_EXPORTER_OTLP_METRICS_PROTOCOL",
    ] {
        cmd.env_remove(key);
    }
}

// Commands that execute a tool replace the proto process with it, so they
// must flush telemetry (and close their spans) before doing so
mod otel {
    use super::*;

    #[test]
    fn exports_spans_for_exec() {
        let sandbox = create_empty_proto_sandbox();
        let collector = Collector::start();

        sandbox
            .run_bin(|cmd| {
                export_to(cmd, &collector);
                cmd.args(["exec", "--", "echo", "hello"]);
            })
            .success();

        assert!(collector.has_span("exec"));
        assert!(collector.has_span("create_command"));
    }

    #[test]
    fn exports_spans_for_run_when_falling_back_to_a_global_tool() {
        let sandbox = create_empty_proto_sandbox();
        let collector = Collector::start();

        sandbox
            .run_bin(|cmd| {
                export_to(cmd, &collector);
                cmd.args(["run", "git", "--", "--version"]);
            })
            .success();

        assert!(collector.has_span("run"));
    }

    #[test]
    fn exports_install_metrics_for_run_when_auto_installing() {
        let sandbox = create_empty_proto_sandbox();
        let collector = Collector::start();

        sandbox
            .run_bin(|cmd| {
                export_to(cmd, &collector);
                cmd.args(["run", "node", "19.0.0", "--", "--version"])
                    .env("PROTO_AUTO_INSTALL", "true");
            })
            .success();

        assert!(collector.has_span("run"));
        assert!(collector.has_metric("proto.tool.install.attempts"));
    }
}
