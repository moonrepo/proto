use proto_core::test_utils::*;
use starbase_sandbox::create_command_with_name;
use starbase_utils::fs;
use std::process::Output;
use std::thread;
use std::time::Duration;

// A config file is shared by every tool, so concurrent pins must merge into it
// instead of overwriting each other. This test holds an exclusive lock on the
// config up front, so that every spawned process is blocked on the
// read-modify-write before any of them can complete it.
mod pin_concurrent {
    use super::*;

    const TOOLS: [(&str, &str); 4] = [
        ("protostar", "1.0.0"),
        ("protoform", "2.0.0"),
        ("moonbase", "3.0.0"),
        ("moonstone", "4.0.0"),
    ];

    fn spawn_pin(sandbox: &ProtoSandbox, id: &str, version: &str) -> thread::JoinHandle<Output> {
        let mut cmd = create_command_with_name(sandbox.path(), "proto", &sandbox.settings);
        cmd.arg("pin").arg(id).arg(version);

        thread::spawn(move || cmd.output().unwrap())
    }

    #[test]
    fn merges_pins_from_every_process() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join(".prototools");

        sandbox.create_file(".prototools", "protojoin = \"5.0.0\"\n");

        // Load the plugin once up front, otherwise the spawned processes spend
        // seconds compiling WASM and reach the lock too far apart to contend
        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("protostar").arg("0.0.1");
            })
            .success();

        let mut lock = fs::lock_file(&config_file).unwrap();

        let children = TOOLS
            .iter()
            .map(|(id, version)| spawn_pin(&sandbox, id, version))
            .collect::<Vec<_>>();

        // Give every process time to start and block on the lock
        thread::sleep(Duration::from_secs(5));

        lock.unlock().unwrap();

        for child in children {
            let output = child.join().unwrap();

            assert!(
                output.status.success(),
                "pin process failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let content = fs::read_file(&config_file).unwrap();

        for (id, version) in TOOLS {
            assert!(
                content.contains(&format!("{id} = \"{version}\"")),
                "missing {id} in config:\n{content}"
            );
        }

        assert!(
            content.contains("protojoin = \"5.0.0\""),
            "missing pre-existing entry in config:\n{content}"
        );

        // Still valid TOML after the concurrent truncate and write
        assert_eq!(load_config(sandbox.path()).versions.len(), TOOLS.len() + 1);
    }
}
