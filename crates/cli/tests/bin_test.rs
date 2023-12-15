mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod bin {
    use super::*;

    #[test]
    fn errors_if_not_installed() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("bin").arg("npm").arg("9.0.0").assert();

        assert.stderr(predicate::str::contains(
            "Unable to find an executable for npm",
        ));
    }

    #[test]
    fn returns_path_if_installed() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("npm")
            .arg("9.0.0")
            .assert()
            .success();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("bin").arg("npm").arg("9.0.0").assert();

        if cfg!(windows) {
            assert.stdout(predicate::str::contains(
                "tools\\npm\\9.0.0\\bin/npm-cli.js",
            ));
        } else {
            assert.stdout(predicate::str::contains("tools/npm/9.0.0/bin/npm-cli.js"));
        }
    }

    #[test]
    fn returns_bin_path() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("npm")
            .arg("9.0.0")
            .assert()
            .success();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("bin").arg("npm").arg("9.0.0").arg("--bin").assert();

        if cfg!(windows) {
            assert.stdout(predicate::str::contains("bin\\npm.cmd"));
        } else {
            assert.stdout(predicate::str::contains("bin/npm"));
        }
    }

    #[test]
    fn returns_shim_path() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("npm")
            .arg("9.0.0")
            .assert()
            .success();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("bin")
            .arg("npm")
            .arg("9.0.0")
            .arg("--shim")
            .assert();

        if cfg!(windows) {
            assert.stdout(predicate::str::contains("shims\\npm.exe"));
        } else {
            assert.stdout(predicate::str::contains("shims/npm"));
        }
    }
}
