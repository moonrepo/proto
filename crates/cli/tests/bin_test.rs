mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod bin {
    use super::*;

    #[test]
    fn errors_if_not_installed() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("node").arg("19.0.0");
        });

        assert.inner.stderr(predicate::str::contains(
            "Unable to find an executable for Node.js",
        ));
    }

    #[test]
    fn returns_path_if_installed() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("19.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("node").arg("19.0.0");
        });

        if cfg!(windows) {
            assert
                .inner
                .stdout(predicate::str::contains("tools\\node\\19.0.0\\node.exe"));
        } else {
            assert
                .inner
                .stdout(predicate::str::contains("tools/node/19.0.0/bin/node"));
        }
    }

    #[test]
    fn returns_bin_path() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("19.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("node").arg("19.0.0").arg("--bin");
        });

        if cfg!(windows) {
            assert.inner.stdout(predicate::str::contains("node.exe"));
        } else {
            assert.inner.stdout(predicate::str::contains("bin/node"));
        }
    }

    #[test]
    fn returns_shim_path() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("19.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("node").arg("19.0.0").arg("--shim");
        });

        if cfg!(windows) {
            assert
                .inner
                .stdout(predicate::str::contains("shims\\node.exe"));
        } else {
            assert.inner.stdout(predicate::str::contains("shims/node"));
        }
    }

    #[test]
    fn returns_exes_dir() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("19.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin")
                .arg("node")
                .arg("19.0.0")
                .arg("--dir")
                .arg("exes");
        });

        if cfg!(windows) {
            assert
                .inner
                .stdout(predicate::str::contains("tools\\node\\19.0.0"));
        } else {
            assert
                .inner
                .stdout(predicate::str::contains("tools/node/19.0.0/bin"));
        }
    }

    #[test]
    fn returns_globals_dir() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("19.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin")
                .arg("node")
                .arg("19.0.0")
                .arg("--dir")
                .arg("globals");
        });

        if cfg!(windows) {
            assert
                .inner
                .stdout(predicate::str::contains("tools\\node\\globals\\bin"));
        } else {
            assert
                .inner
                .stdout(predicate::str::contains("tools/node/globals/bin"));
        }
    }
}
