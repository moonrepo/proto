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
        shim.write_stdin("foo bar baz");

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
}
