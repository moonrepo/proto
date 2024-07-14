mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod bin {
    use super::*;

    #[test]
    fn errors_if_not_installed() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("npm").arg("9.0.0");
        });

        assert.inner.stderr(predicate::str::contains(
            "Unable to find an executable for npm",
        ));
    }

    #[test]
    fn returns_path_if_installed() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("npm").arg("9.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("npm").arg("9.0.0");
        });

        if cfg!(windows) {
            assert.inner.stdout(predicate::str::contains(
                "tools\\npm\\9.0.0\\bin/npm-cli.js",
            ));
        } else {
            assert
                .inner
                .stdout(predicate::str::contains("tools/npm/9.0.0/bin/npm-cli.js"));
        }
    }

    #[test]
    fn returns_bin_path() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("npm").arg("9.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("npm").arg("9.0.0").arg("--bin");
        });

        if cfg!(windows) {
            assert
                .inner
                .stdout(predicate::str::contains("bin/npm-cli.js"));
        } else {
            assert.inner.stdout(predicate::str::contains("bin/npm"));
        }
    }

    #[test]
    fn returns_shim_path() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("npm").arg("9.0.0");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("npm").arg("9.0.0").arg("--shim");
        });

        if cfg!(windows) {
            assert
                .inner
                .stdout(predicate::str::contains("shims\\npm.exe"));
        } else {
            assert.inner.stdout(predicate::str::contains("shims/npm"));
        }
    }
}
