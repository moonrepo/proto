use proto_core::test_utils::*;
use proto_core::warpgate::{PluginLocator, find_debug_locator};
use proto_core::{Id, LockRecord, ProtoLock, UnresolvedVersionSpec, VersionSpec};
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use system_env::SystemPlatform;

struct PrebuiltServer {
    requests: Arc<AtomicUsize>,
    url: String,
}

impl PrebuiltServer {
    fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

// Serve each request with its path as the response body, so that every
// platform and version downloads a different file (and has a different
// checksum). Paths for riscv64 are not found, to mimic a missing pre-built.
fn spawn_prebuilt_server() -> PrebuiltServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&requests);

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

            counter.fetch_add(1, Ordering::SeqCst);

            let request = String::from_utf8_lossy(&request);
            let path = request.split(' ').nth(1).unwrap_or_default().to_owned();
            let status = if path.contains("riscv64") {
                "404 Not Found"
            } else {
                "200 OK"
            };

            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{path}",
                path.len(),
            );
        }
    });

    PrebuiltServer {
        requests,
        url: format!("http://{addr}"),
    }
}

fn get_plugin_file(name: &str) -> String {
    match find_debug_locator(name)
        .expect("Test plugins not available. Run `just build-wasm` to build them!")
    {
        PluginLocator::File(file) => file.file.replace('\\', "/"),
        _ => unreachable!(),
    }
}

fn create_lock_sandbox(server: &PrebuiltServer, lockfile: bool) -> ProtoSandbox {
    let sandbox = create_empty_proto_sandbox();

    sandbox.create_file(
        ".prototools",
        format!(
            r#"
api-usage = "1"

[plugins.tools]
api-usage = "{}"

[settings]
lockfile = {lockfile}
builtin-backends = false
builtin-tools = false

[tools.api-usage]
prebuilt-url = "{}"
"#,
            get_plugin_file("proto_api_usage"),
            server.url,
        ),
    );

    sandbox
}

fn host_platform() -> SystemPlatform {
    SystemPlatform::from_env()
}

fn load_records(sandbox: &ProtoSandbox) -> Vec<LockRecord> {
    ProtoLock::load_from(sandbox.path())
        .unwrap()
        .tools
        .remove(&Id::raw("api-usage"))
        .unwrap_or_default()
}

fn get_platforms(records: &[LockRecord]) -> BTreeSet<SystemPlatform> {
    records
        .iter()
        .map(|record| record.get_platform().unwrap())
        .collect()
}

// The test plugin logs to stdout when registering, so skip to the JSON
fn parse_items(stdout: &str) -> Vec<serde_json::Value> {
    let start = stdout.find("\n[").map(|index| index + 1).unwrap_or(0);

    serde_json::from_str(&stdout[start..]).unwrap()
}

fn platform(value: &str) -> SystemPlatform {
    SystemPlatform::parse(value).unwrap()
}

mod lock {
    use super::*;

    #[test]
    fn locks_the_host_platform_by_default() {
        let server = spawn_prebuilt_server();
        let sandbox = create_lock_sandbox(&server, true);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("lock").arg("--json");
        });

        let items = parse_items(&assert.stdout());
        assert.success();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["tool"], "api-usage");
        assert_eq!(items[0]["spec"], "~1");
        assert_eq!(items[0]["version"], "1.1.0");
        assert_eq!(items[0]["platform"], host_platform().to_string());
        assert_eq!(items[0]["status"], "locked");
        assert!(items[0]["checksum"].is_string());

        let records = load_records(&sandbox);

        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].spec,
            Some(UnresolvedVersionSpec::parse("1").unwrap())
        );
        assert_eq!(
            records[0].version,
            Some(VersionSpec::parse("1.1.0").unwrap())
        );
        assert!(records[0].checksum.is_some());
        assert_eq!(get_platforms(&records), BTreeSet::from([host_platform()]));

        // Nothing is installed
        assert!(!sandbox.path().join(".proto/tools/api-usage/1.1.0").exists());
    }

    #[test]
    fn locks_the_provided_platforms() {
        let server = spawn_prebuilt_server();
        let sandbox = create_lock_sandbox(&server, true);

        // Both formats are supported, and duplicates are ignored
        sandbox
            .run_bin(|cmd| {
                cmd.arg("lock")
                    .arg("--platform")
                    .arg("x64-linux,linux-x64,linux-arm64-musl")
                    .arg("--platform")
                    .arg("arm64-macos");
            })
            .success();

        let records = load_records(&sandbox);

        assert_eq!(records.len(), 3);
        assert_eq!(
            get_platforms(&records),
            BTreeSet::from([
                platform("x64-linux-gnu"),
                platform("arm64-linux-musl"),
                platform("arm64-macos"),
            ])
        );

        let checksums = records
            .iter()
            .map(|record| record.checksum.clone().unwrap().to_string())
            .collect::<BTreeSet<_>>();

        assert_eq!(checksums.len(), 3);
        assert_eq!(server.requests(), 3);
    }

    #[test]
    fn locks_platforms_that_already_have_a_record() {
        let server = spawn_prebuilt_server();
        let sandbox = create_lock_sandbox(&server, true);

        // A record without a checksum, like after `outdated --update`
        let mut record = LockRecord {
            spec: Some(UnresolvedVersionSpec::parse("1").unwrap()),
            version: Some(VersionSpec::parse("1.1.0").unwrap()),
            ..Default::default()
        };
        record.set_platform(&platform("x64-linux-musl"));

        let mut lock = ProtoLock::default();
        lock.tools.insert(Id::raw("api-usage"), vec![record]);
        lock.path = sandbox.path().join(".protolock");
        lock.save().unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("lock");
            })
            .success();

        let records = load_records(&sandbox);

        assert!(records.iter().all(|record| record.checksum.is_some()));
        assert_eq!(
            get_platforms(&records),
            BTreeSet::from([host_platform(), platform("x64-linux-musl")])
        );
    }

    #[test]
    fn doesnt_download_platforms_that_are_already_locked() {
        let server = spawn_prebuilt_server();
        let sandbox = create_lock_sandbox(&server, true);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("lock").arg("--platform").arg("x64-linux");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("lock").arg("--platform").arg("x64-linux");
        });

        let output = format!("{}{}", assert.stdout(), assert.stderr());
        assert.success();

        assert!(output.contains("Already locked"), "{output}");
        assert!(
            output.contains("Everything has already been locked!"),
            "{output}"
        );
        assert_eq!(server.requests(), 1);
    }

    #[test]
    fn force_downloads_platforms_that_are_already_locked() {
        let server = spawn_prebuilt_server();
        let sandbox = create_lock_sandbox(&server, true);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("lock").arg("--platform").arg("x64-linux");
            })
            .success();

        let before = load_records(&sandbox);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("lock")
                .arg("--platform")
                .arg("x64-linux")
                .arg("--force")
                .arg("--json");
        });

        let items = parse_items(&assert.stdout());
        assert.success();

        assert_eq!(items[0]["status"], "locked");
        assert_eq!(load_records(&sandbox), before);
        assert_eq!(server.requests(), 2);
    }

    #[test]
    fn fails_platforms_that_cant_be_locked() {
        let server = spawn_prebuilt_server();
        let sandbox = create_lock_sandbox(&server, true);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("lock")
                .arg("--platform")
                .arg("x64-linux,riscv64-linux");
        });

        let output = format!("{}{}", assert.stdout(), assert.stderr());
        assert.code(1);

        assert!(
            output.contains("Failed to lock api-usage ~1 for riscv64-linux-gnu"),
            "{output}"
        );

        // Other platforms are still locked
        assert_eq!(
            get_platforms(&load_records(&sandbox)),
            BTreeSet::from([platform("x64-linux-gnu")])
        );
    }

    #[test]
    fn reports_failures_in_json() {
        let server = spawn_prebuilt_server();
        let sandbox = create_lock_sandbox(&server, true);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("lock")
                .arg("--platform")
                .arg("riscv64-linux")
                .arg("--json");
        });

        let items = parse_items(&assert.stdout());
        assert.code(1);

        assert_eq!(items[0]["status"], "failed");
        assert!(
            items[0]["error"]
                .as_str()
                .unwrap()
                .contains("riscv64-linux-gnu.txt")
        );
    }

    #[test]
    fn locks_a_single_tool() {
        let server = spawn_prebuilt_server();
        let sandbox = create_lock_sandbox(&server, true);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("lock").arg("api-usage").arg("1.0").arg("--json");
        });

        let items = parse_items(&assert.stdout());
        assert.success();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["version"], "1.0.0");
    }

    #[test]
    fn errors_when_lockfile_is_not_enabled() {
        let server = spawn_prebuilt_server();
        let sandbox = create_lock_sandbox(&server, false);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("lock");
            })
            .failure()
            .stderr(predicate::str::contains("a lockfile has not been enabled"));

        assert_eq!(server.requests(), 0);
    }

    #[test]
    fn errors_when_lockfile_is_immutable() {
        let server = spawn_prebuilt_server();
        let sandbox = create_lock_sandbox(&server, true);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("lock").env("PROTO_IMMUTABLE_LOCKFILE", "true");
            })
            .failure()
            .stderr(predicate::str::contains("the lockfile is immutable"));

        assert!(!sandbox.path().join(".protolock").exists());
    }

    #[test]
    fn skips_tools_that_install_natively() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            format!(
                r#"
protostar = "1.0.0"

[plugins.tools]
protostar = "{}"

[settings]
lockfile = true
builtin-backends = false
builtin-tools = false
"#,
                get_plugin_file("proto_mocked_tool")
            ),
        );

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("lock").arg("--json");
        });

        let items = parse_items(&assert.stdout());
        assert.success();

        assert_eq!(items[0]["tool"], "protostar");
        assert_eq!(items[0]["status"], "skipped");
        assert_eq!(items[0]["reason"], "native-install");
    }
}
