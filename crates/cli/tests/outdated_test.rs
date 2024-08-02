mod utils;

use starbase_sandbox::assert_debug_snapshot;
use starbase_sandbox::predicates::prelude::*;
use std::fs;
use utils::*;

mod outdated {
    use super::*;

    #[test]
    fn errors_when_nothing_configured() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated");
        });

        assert
            .inner
            .stderr(predicate::str::contains("No tools have been configured"));
    }

    #[test]
    fn reports_all_non_global_configs() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/.prototools", r#"go = "*""#);
        sandbox.create_file("a/.prototools", r#"node = "*""#);
        sandbox.create_file("a/b/.prototools", r#"npm = "*""#);
        sandbox.create_file("a/b/c/.prototools", r#"bun = "*""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated")
                .current_dir(sandbox.path().join("a/b/c"));
        });

        let output = assert.output();

        assert!(predicate::str::contains("node").eval(&output));
        assert!(predicate::str::contains("npm").eval(&output));
        assert!(predicate::str::contains("bun").eval(&output));
        assert!(predicate::str::contains("go").not().eval(&output));
    }

    #[test]
    fn only_includes_local_config() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/.prototools", r#"go = "*""#);
        sandbox.create_file("a/.prototools", r#"node = "*""#);
        sandbox.create_file("a/b/.prototools", r#"npm = "*""#);
        sandbox.create_file("a/b/c/.prototools", r#"bun = "*""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated")
                .arg("--config-mode")
                .arg("local")
                .current_dir(sandbox.path().join("a/b/c"));
        });

        let output = assert.output();

        assert!(predicate::str::contains("node").not().eval(&output));
        assert!(predicate::str::contains("npm").not().eval(&output));
        assert!(predicate::str::contains("bun").eval(&output));
        assert!(predicate::str::contains("go").not().eval(&output));
    }

    #[test]
    fn can_include_global_config() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/.prototools", r#"go = "*""#);
        sandbox.create_file("a/.prototools", r#"node = "*""#);
        sandbox.create_file("a/b/.prototools", r#"npm = "*""#);
        sandbox.create_file("a/b/c/.prototools", r#"bun = "*""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated")
                .arg("--config-mode")
                .arg("upwards-global")
                .current_dir(sandbox.path().join("a/b/c"));
        });

        let output = assert.output();

        assert!(predicate::str::contains("node").eval(&output));
        assert!(predicate::str::contains("npm").eval(&output));
        assert!(predicate::str::contains("bun").eval(&output));
        assert!(predicate::str::contains("go").eval(&output));
    }

    #[test]
    fn global_doesnt_overwrite_local() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/.prototools", r#"node = "18""#);
        sandbox.create_file("a/.prototools", r#"node = "20""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated")
                .arg("--config-mode")
                .arg("upwards-global")
                .current_dir(sandbox.path().join("a"));
        });

        let output = assert.output();

        assert!(predicate::str::contains("node").eval(&output));
        assert!(predicate::str::contains("20").eval(&output));
    }

    #[test]
    fn updates_each_file_respectively() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/.prototools", r#"go = "1.19""#);
        sandbox.create_file("a/.prototools", r#"node = "19.0.0""#);
        sandbox.create_file("a/b/.prototools", r#"npm = "9.0.0""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("outdated")
                    .arg("--update")
                    .arg("--config-mode")
                    .arg("upwards-global")
                    .current_dir(sandbox.path().join("a/b"));
            })
            .success();

        assert_debug_snapshot!(vec![
            fs::read_to_string(sandbox.path().join(".proto/.prototools")).unwrap(),
            fs::read_to_string(sandbox.path().join("a/.prototools")).unwrap(),
            fs::read_to_string(sandbox.path().join("a/b/.prototools")).unwrap(),
        ]);
    }

    #[test]
    fn can_update_with_latest_version() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/.prototools", r#"pnpm = "6.0.0""#);
        sandbox.create_file("a/.prototools", r#"node = "19.0.0""#);
        sandbox.create_file("a/b/.prototools", r#"npm = "8.0.0""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("outdated")
                    .arg("--update")
                    .arg("--config-mode")
                    .arg("upwards-global")
                    .arg("--latest")
                    .current_dir(sandbox.path().join("a/b"));
            })
            .success();

        assert!(
            !fs::read_to_string(sandbox.path().join(".proto/.prototools"))
                .unwrap()
                .contains("6.0.0")
        );
        assert!(!fs::read_to_string(sandbox.path().join("a/.prototools"))
            .unwrap()
            .contains("19.0.0"));
        assert!(!fs::read_to_string(sandbox.path().join("a/b/.prototools"))
            .unwrap()
            .contains("8.0.0"));
    }
}
