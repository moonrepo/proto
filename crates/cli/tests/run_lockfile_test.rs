use proto_core::ProtoLock;
use proto_core::test_utils::*;
use starbase_sandbox::Sandbox;
use starbase_sandbox::predicates::prelude::*;
use system_env::{SystemArch, SystemOS};

fn create_record(spec: &str, version: &str, checksum: Option<&str>) -> String {
    let mut record = format!(
        r#"
[[tools.protostar]]
os = "{}"
arch = "{}"
spec = "{spec}"
version = "{version}"
"#,
        SystemOS::default(),
        SystemArch::default()
    );

    if let Some(checksum) = checksum {
        record.push_str(&format!("checksum = \"{checksum}\"\n"));
    }

    record
}

// A record that could only have been created on another machine,
// as proto does not run on this platform combination
fn create_other_platform_record(spec: &str, version: &str) -> String {
    format!(
        r#"
[[tools.protostar]]
os = "solaris"
arch = "sparc64"
spec = "{spec}"
version = "{version}"
"#
    )
}

fn is_installed(sandbox: &Sandbox, version: &str) -> bool {
    sandbox
        .path()
        .join(".proto/tools/protostar")
        .join(version)
        .exists()
}

// The mocked tool installs empty files as its executables, so running the
// tool itself always fails, and these tests only cover the auto-install
mod run_lockfile {
    use super::*;

    mod auto_install {
        use super::*;

        #[test]
        fn errors_for_checksum_mismatch() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                create_record("5.10.0", "5.10.0", Some("sha256:invalid")),
            );

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("run")
                        .arg("protostar")
                        .arg("5.10.0")
                        .env("PROTO_AUTO_INSTALL", "true");
                })
                .failure();

            assert.stderr(predicate::str::contains("Checksum mismatch"));
            assert!(!is_installed(&sandbox, "5.10.0"));
        }

        #[test]
        fn installs_unlocked_version_when_not_immutable() {
            let sandbox = create_proto_sandbox("lockfile");

            sandbox.run_bin(|cmd| {
                cmd.arg("run")
                    .arg("protostar")
                    .arg("5.10.0")
                    .env("PROTO_AUTO_INSTALL", "true");
            });

            assert!(is_installed(&sandbox, "5.10.0"));
        }
    }

    mod auto_install_immutable {
        use super::*;

        #[test]
        fn installs_from_locked_record_without_modifying_lockfile() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(".protolock", create_record("^5.10", "5.10.10", None));

            let assert = sandbox.run_bin(|cmd| {
                // 5.10.15 is the latest, but the record pins 5.10.10
                cmd.arg("run")
                    .arg("protostar")
                    .arg("^5.10")
                    .env("PROTO_AUTO_INSTALL", "true")
                    .env("PROTO_IMMUTABLE_LOCKFILE", "true");
            });

            assert!(!assert.stderr().contains("Lockfile is immutable"));
            assert!(is_installed(&sandbox, "5.10.10"));

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert!(records[0].checksum.is_none());
        }

        #[test]
        fn errors_when_record_is_missing() {
            let sandbox = create_proto_sandbox("lockfile");

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("run")
                        .arg("protostar")
                        .arg("5.10.0")
                        .env("PROTO_AUTO_INSTALL", "true")
                        .env("PROTO_IMMUTABLE_LOCKFILE", "true");
                })
                .failure();

            assert.stderr(predicate::str::contains(
                "Lockfile is immutable, but is missing a record",
            ));
            assert!(!is_installed(&sandbox, "5.10.0"));
            assert!(!sandbox.path().join(".protolock").exists());
        }

        #[test]
        fn errors_when_only_other_platforms_are_locked() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                create_other_platform_record("5.10.0", "5.10.0"),
            );

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("run")
                        .arg("protostar")
                        .arg("5.10.0")
                        .env("PROTO_AUTO_INSTALL", "true")
                        .env("PROTO_IMMUTABLE_LOCKFILE", "true");
                })
                .failure();

            assert.stderr(predicate::str::contains(
                "Only other platforms have locked this version",
            ));
            assert!(!is_installed(&sandbox, "5.10.0"));
        }

        #[test]
        fn doesnt_require_a_record_when_already_installed() {
            let sandbox = create_proto_sandbox("lockfile");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("5.10.0");
                })
                .success();

            // Nothing is downloaded, so there's nothing to verify
            std::fs::remove_file(sandbox.path().join(".protolock")).unwrap();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("run")
                    .arg("protostar")
                    .arg("5.10.0")
                    .env("PROTO_AUTO_INSTALL", "true")
                    .env("PROTO_IMMUTABLE_LOCKFILE", "true");
            });

            assert!(!assert.stderr().contains("Lockfile is immutable"));
        }
    }
}
