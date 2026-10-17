use proto_core::flow::install::{InstallOptions, Installer, ProtoInstallError};
use proto_core::flow::lock::{LockOutcome, LockSkipReason};
use proto_core::{
    Id, LockRecord, ProtoEnvironment, ProtoLock, Tool, ToolContext, ToolSpec,
    load_tool_from_locator,
};
use proto_pdk_api::{Checksum, InstallStrategy};
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use system_env::{SystemArch, SystemLibc, SystemOS, SystemPlatform};
use version_spec::{UnresolvedVersionSpec, VersionSpec};

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

fn create_sandbox(server: &PrebuiltServer, lockfile: bool) -> Sandbox {
    let sandbox = create_empty_sandbox();

    sandbox.create_file(
        ".prototools",
        format!(
            r#"
api-usage = "1"
protostar = "1"

[settings]
lockfile = {lockfile}

[tools.api-usage]
prebuilt-url = "{}"
"#,
            server.url
        ),
    );

    sandbox
}

fn create_proto(sandbox: &Sandbox) -> ProtoEnvironment {
    let mut proto = ProtoEnvironment::new_testing(sandbox.path()).unwrap();
    proto.working_dir = sandbox.path().to_path_buf();
    proto
}

async fn load_tool_with_plugin(proto: &ProtoEnvironment, id: &str, plugin: &str) -> Tool {
    load_tool_from_locator(
        ToolContext::parse(id).unwrap(),
        proto,
        warpgate::find_debug_locator(plugin)
            .expect("Test plugins not available. Run `just build-wasm` to build them!"),
    )
    .await
    .unwrap()
}

// Tools for every platform must be derived from the same environment,
// so that they share the same lockfile state, like the lock command
async fn load_tool(proto: &ProtoEnvironment, platform: &str) -> Tool {
    load_tool_with_plugin(
        &proto.for_platform(SystemPlatform::parse(platform).unwrap()),
        "api-usage",
        "proto_api_usage",
    )
    .await
}

fn resolved_spec(req: &str, version: &str) -> ToolSpec {
    let mut spec = ToolSpec::parse(req).unwrap();
    spec.resolve(VersionSpec::parse(version).unwrap());
    spec
}

async fn lock(tool: &Tool, spec: &ToolSpec) -> Result<LockOutcome, ProtoInstallError> {
    lock_with_options(tool, spec, InstallOptions::default()).await
}

async fn lock_with_options(
    tool: &Tool,
    spec: &ToolSpec,
    options: InstallOptions,
) -> Result<LockOutcome, ProtoInstallError> {
    Installer::new(tool, spec).lock(options).await
}

fn load_records(sandbox: &Sandbox) -> Vec<LockRecord> {
    ProtoLock::load_from(sandbox.path())
        .unwrap()
        .tools
        .remove(&Id::raw("api-usage"))
        .unwrap_or_default()
}

fn save_records(sandbox: &Sandbox, records: Vec<LockRecord>) {
    let mut lock = ProtoLock::load_from(sandbox.path()).unwrap();
    lock.tools.insert(Id::raw("api-usage"), records);
    lock.save().unwrap();
}

fn make_record(version: &str, platform: &str) -> LockRecord {
    let mut record = LockRecord {
        spec: Some(UnresolvedVersionSpec::parse("1").unwrap()),
        version: Some(VersionSpec::parse(version).unwrap()),
        checksum: Some(Checksum::sha256("existing".into())),
        ..Default::default()
    };
    record.set_platform(&SystemPlatform::parse(platform).unwrap());
    record
}

fn unwrap_locked(outcome: LockOutcome) -> LockRecord {
    match outcome {
        LockOutcome::Locked(record) => record,
        other => panic!("Expected a locked outcome, received {other:?}"),
    }
}

mod installer_lock {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn locks_each_platform_with_its_own_checksum() {
        let server = spawn_prebuilt_server();
        let sandbox = create_sandbox(&server, true);
        let proto = create_proto(&sandbox);
        let spec = resolved_spec("1", "1.0.0");

        // The plugin downloads `<version>/<arch>-<os>-<libc>.txt`
        for (platform, file) in [
            ("x64-linux", "x64-linux-gnu.txt"),
            ("x64-linux-musl", "x64-linux-musl.txt"),
            ("arm64-macos", "arm64-macos-unknown.txt"),
        ] {
            let tool = load_tool(&proto, platform).await;
            let record = unwrap_locked(lock(&tool, &spec).await.unwrap());

            assert!(record.checksum.is_some());
            assert_eq!(record.source, Some(format!("{}/1.0.0/{file}", server.url)));

            // Nothing is installed
            assert!(!tool.get_product_dir(&spec).exists());
        }

        let records = load_records(&sandbox);

        assert_eq!(records.len(), 3);
        assert_eq!(server.requests(), 3);

        for record in &records {
            assert_eq!(
                record.spec,
                Some(UnresolvedVersionSpec::parse("1").unwrap())
            );
            assert_eq!(record.version, Some(VersionSpec::parse("1.0.0").unwrap()));
            assert!(record.source.is_none());
        }

        let platforms = records
            .iter()
            .map(|record| (record.os, record.arch, record.libc))
            .collect::<Vec<_>>();

        assert!(platforms.contains(&(
            Some(SystemOS::Linux),
            Some(SystemArch::X64),
            Some(SystemLibc::Gnu)
        )));
        assert!(platforms.contains(&(
            Some(SystemOS::Linux),
            Some(SystemArch::X64),
            Some(SystemLibc::Musl)
        )));
        assert!(platforms.contains(&(Some(SystemOS::MacOS), Some(SystemArch::Arm64), None)));

        let checksums = records
            .iter()
            .map(|record| record.checksum.clone().unwrap().to_string())
            .collect::<BTreeSet<_>>();

        assert_eq!(checksums.len(), 3);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_download_when_already_locked() {
        let server = spawn_prebuilt_server();
        let sandbox = create_sandbox(&server, true);
        let proto = create_proto(&sandbox);
        let tool = load_tool(&proto, "x64-linux").await;
        let spec = resolved_spec("1", "1.0.0");

        let record = unwrap_locked(lock(&tool, &spec).await.unwrap());

        assert_eq!(
            lock(&tool, &spec).await.unwrap(),
            LockOutcome::AlreadyLocked(record.for_lockfile())
        );
        assert_eq!(server.requests(), 1);
    }

    // The download may be served from the HTTP cache, like installs
    #[tokio::test(flavor = "multi_thread")]
    async fn force_downloads_and_verifies_the_record() {
        let server = spawn_prebuilt_server();
        let sandbox = create_sandbox(&server, true);
        let proto = create_proto(&sandbox);
        let tool = load_tool(&proto, "x64-linux").await;
        let spec = resolved_spec("1", "1.0.0");

        let first = unwrap_locked(lock(&tool, &spec).await.unwrap());
        let second = unwrap_locked(
            lock_with_options(
                &tool,
                &spec,
                InstallOptions {
                    force: true,
                    ..Default::default()
                },
            )
            .await
            .unwrap(),
        );

        assert_eq!(first.checksum, second.checksum);
        assert_eq!(load_records(&sandbox).len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn force_errors_when_checksum_doesnt_match_the_record() {
        let server = spawn_prebuilt_server();
        let sandbox = create_sandbox(&server, true);

        save_records(&sandbox, vec![make_record("1.0.0", "x64-linux")]);

        let proto = create_proto(&sandbox);
        let tool = load_tool(&proto, "x64-linux").await;
        let spec = resolved_spec("1", "1.0.0");

        let error = lock_with_options(
            &tool,
            &spec,
            InstallOptions {
                force: true,
                ..Default::default()
            },
        )
        .await
        .unwrap_err()
        .to_string();

        assert!(error.contains("Checksum mismatch"), "{error}");

        // The record is left untouched
        assert_eq!(
            load_records(&sandbox),
            vec![make_record("1.0.0", "x64-linux")]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn replaces_record_for_another_version() {
        let server = spawn_prebuilt_server();
        let sandbox = create_sandbox(&server, true);

        // A higher version, which an install would not replace
        save_records(&sandbox, vec![make_record("1.2.0", "x64-linux")]);

        let proto = create_proto(&sandbox);
        let tool = load_tool(&proto, "x64-linux").await;
        let record = unwrap_locked(lock(&tool, &resolved_spec("1", "1.0.0")).await.unwrap());

        assert_eq!(load_records(&sandbox), vec![record.for_lockfile()]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn replaces_record_without_a_libc() {
        let server = spawn_prebuilt_server();
        let sandbox = create_sandbox(&server, true);

        let mut legacy = make_record("1.0.0", "x64-linux");
        legacy.libc = None;

        save_records(&sandbox, vec![legacy]);

        let proto = create_proto(&sandbox);
        let tool = load_tool(&proto, "x64-linux").await;
        let record = unwrap_locked(lock(&tool, &resolved_spec("1", "1.0.0")).await.unwrap());

        assert_eq!(record.libc, Some(SystemLibc::Gnu));
        assert_eq!(load_records(&sandbox), vec![record.for_lockfile()]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn keeps_records_for_other_platforms() {
        let server = spawn_prebuilt_server();
        let sandbox = create_sandbox(&server, true);
        let other = make_record("1.0.0", "x64-linux-musl");

        save_records(&sandbox, vec![other.clone()]);

        let proto = create_proto(&sandbox);
        let tool = load_tool(&proto, "x64-linux").await;

        unwrap_locked(lock(&tool, &resolved_spec("1", "1.0.0")).await.unwrap());

        let records = load_records(&sandbox);

        assert_eq!(records.len(), 2);
        assert!(records.contains(&other));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn errors_when_the_download_fails() {
        let server = spawn_prebuilt_server();
        let sandbox = create_sandbox(&server, true);
        let proto = create_proto(&sandbox);
        let tool = load_tool(&proto, "riscv64-linux").await;

        let error = lock(&tool, &resolved_spec("1", "1.0.0"))
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("riscv64-linux-gnu.txt"), "{error}");
        assert!(load_records(&sandbox).is_empty());

        // The staged download is removed
        assert!(!tool.get_temp_dir().join("lock").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_when_lockfile_is_disabled() {
        let server = spawn_prebuilt_server();
        let sandbox = create_sandbox(&server, false);
        let proto = create_proto(&sandbox);
        let tool = load_tool(&proto, "x64-linux").await;

        assert_eq!(
            lock(&tool, &resolved_spec("1", "1.0.0")).await.unwrap(),
            LockOutcome::Skipped(LockSkipReason::NoLockfile)
        );
        assert_eq!(server.requests(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_when_building_from_source() {
        let server = spawn_prebuilt_server();
        let sandbox = create_sandbox(&server, true);
        let proto = create_proto(&sandbox);
        let tool = load_tool(&proto, "x64-linux").await;

        assert_eq!(
            lock_with_options(
                &tool,
                &resolved_spec("1", "1.0.0"),
                InstallOptions {
                    strategy: InstallStrategy::BuildFromSource,
                    ..Default::default()
                },
            )
            .await
            .unwrap(),
            LockOutcome::Skipped(LockSkipReason::BuildFromSource)
        );
        assert_eq!(server.requests(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_when_installed_natively() {
        let server = spawn_prebuilt_server();
        let sandbox = create_sandbox(&server, true);
        let proto = create_proto(&sandbox);
        let tool = load_tool_with_plugin(&proto, "protostar", "proto_mocked_tool").await;

        assert_eq!(
            lock(&tool, &resolved_spec("1", "1.0.0")).await.unwrap(),
            LockOutcome::Skipped(LockSkipReason::NativeInstall)
        );
    }
}
