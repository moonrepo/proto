mod utils;

use starbase_sandbox::{assert_snapshot, get_assert_output};
use std::path::PathBuf;
use utils::*;

mod shim_bin {
    use super::*;

    fn get_fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
    }

    #[test]
    fn standard_output() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("--pin")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        let mut shim = create_shim_command(sandbox.path(), "node");
        shim.arg(get_fixture("tests/fixtures/shim-standard.mjs"));
        shim.env_remove("PROTO_LOG");

        let assert = shim.assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn waits_for_timeout() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("--pin")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        let mut shim = create_shim_command(sandbox.path(), "node");
        shim.arg(get_fixture("tests/fixtures/shim-timeout.mjs"));
        shim.env_remove("PROTO_LOG");

        let assert = shim.assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn waits_for_top_level_await() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("--pin")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        let mut shim = create_shim_command(sandbox.path(), "node");
        shim.arg(get_fixture("tests/fixtures/shim-tla.mjs"));
        shim.env_remove("PROTO_LOG");

        let assert = shim.assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_stdin_piped_data() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("--pin")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        let mut shim = create_shim_command(sandbox.path(), "node");
        shim.arg(get_fixture("tests/fixtures/shim-piped-stdin.mjs"));
        shim.env_remove("PROTO_LOG");
        shim.write_stdin("this data comes from stdin");

        let assert = shim.assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_file_piped_data() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("--pin")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        let mut shim = create_shim_command(sandbox.path(), "node");
        shim.arg(get_fixture("tests/fixtures/shim-piped-stdin.mjs"));
        shim.env_remove("PROTO_LOG");
        shim.pipe_stdin("tests/fixtures/piped-data.txt").unwrap();

        let assert = shim.assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_exit_codes() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("--pin")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        let mut shim = create_shim_command(sandbox.path(), "node");
        shim.arg(get_fixture("tests/fixtures/shim-code-0.mjs"));
        shim.assert().code(0);

        let mut shim = create_shim_command(sandbox.path(), "node");
        shim.arg(get_fixture("tests/fixtures/shim-code-1.mjs"));
        shim.assert().code(1);
    }

    #[test]
    #[cfg(not(windows))]
    fn handles_signals() {
        use shared_child::unix::SharedChildExt;
        use shared_child::SharedChild;
        use std::os::unix::process::ExitStatusExt;

        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("--pin")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        let mut shim = create_shim_command_std(sandbox.path(), "node");
        shim.arg(get_fixture("tests/fixtures/shim-signal.mjs"));
        shim.env_remove("PROTO_LOG");

        // Interrupt / SIGINT
        let child = SharedChild::spawn(&mut shim).unwrap();
        child.send_signal(2).unwrap();

        assert_eq!(child.wait().unwrap().signal().unwrap(), 2);

        // Terminate / SIGTERM
        let child = SharedChild::spawn(&mut shim).unwrap();
        child.send_signal(15).unwrap();

        assert_eq!(child.wait().unwrap().signal().unwrap(), 15);

        // Hangup / SIGHUP
        let child = SharedChild::spawn(&mut shim).unwrap();
        child.send_signal(1).unwrap();

        assert_eq!(child.wait().unwrap().signal().unwrap(), 1);
    }
}
