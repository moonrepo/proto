mod utils;

use starbase_sandbox::predicates::prelude::*;
use std::env;
use utils::*;

mod npm {
    use super::*;

    #[test]
    fn errors_if_installing_global() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("npm")
            .arg("latest")
            .assert()
            .success();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("npm")
            .arg("latest")
            .args(["--", "install", "-g", "typescript"])
            .assert();

        assert.stderr(predicate::str::contains(
            "Global binaries must be installed with proto install-global npm",
        ));
    }

    #[test]
    fn can_bypass_global_check() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("npm")
            .arg("latest")
            .assert()
            .success();

        env::set_var("PROTO_NODE_INTERCEPT_GLOBALS", "0");

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("npm")
            .arg("latest")
            .args(["--", "install", "-g", "typescript"])
            .assert();

        env::remove_var("PROTO_NODE_INTERCEPT_GLOBALS");

        assert.stderr(
            predicate::str::contains(
                "Global binaries must be installed with proto install-global npm",
            )
            .not(),
        );
    }
}

mod pnpm {
    use super::*;

    #[test]
    fn errors_if_installing_global() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install").arg("pnpm").assert().success();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("pnpm")
            .args(["--", "add", "-g", "typescript"])
            .assert();

        assert.stderr(predicate::str::contains(
            "Global binaries must be installed with proto install-global pnpm",
        ));
    }
}

mod yarn {
    use super::*;

    #[test]
    fn errors_if_installing_global() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install").arg("yarn").assert().success();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("yarn")
            .args(["--", "global", "add", "typescript"])
            .assert();

        assert.stderr(predicate::str::contains(
            "Global binaries must be installed with proto install-global yarn",
        ));
    }
}
