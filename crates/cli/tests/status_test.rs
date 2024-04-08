mod utils;

use starbase_sandbox::get_assert_output;
use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod status {
    use super::*;

    #[test]
    fn errors_when_nothing_configured() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("status").assert().failure();

        assert.stderr(predicate::str::contains("No tools have been configured"));
    }

    #[test]
    fn reports_all_non_global_configs() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".proto/.prototools", r#"go = "*""#);
        sandbox.create_file("a/.prototools", r#"node = "*""#);
        sandbox.create_file("a/b/.prototools", r#"npm = "*""#);
        sandbox.create_file("a/b/c/.prototools", r#"bun = "*""#);

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("status")
            .current_dir(sandbox.path().join("a/b/c"))
            .assert()
            .success();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("node").eval(&output));
        assert!(predicate::str::contains("npm").eval(&output));
        assert!(predicate::str::contains("bun").eval(&output));
        assert!(predicate::str::contains("go").not().eval(&output));
    }

    #[test]
    fn only_includes_local_config() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".proto/.prototools", r#"go = "*""#);
        sandbox.create_file("a/.prototools", r#"node = "*""#);
        sandbox.create_file("a/b/.prototools", r#"npm = "*""#);
        sandbox.create_file("a/b/c/.prototools", r#"bun = "*""#);

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("status")
            .arg("--only-local")
            .current_dir(sandbox.path().join("a/b/c"))
            .assert()
            .success();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("node").not().eval(&output));
        assert!(predicate::str::contains("npm").not().eval(&output));
        assert!(predicate::str::contains("bun").eval(&output));
        assert!(predicate::str::contains("go").not().eval(&output));
    }

    #[test]
    fn can_include_global_config() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".proto/.prototools", r#"go = "*""#);
        sandbox.create_file("a/.prototools", r#"node = "*""#);
        sandbox.create_file("a/b/.prototools", r#"npm = "*""#);
        sandbox.create_file("a/b/c/.prototools", r#"bun = "*""#);

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("status")
            .arg("--include-global")
            .current_dir(sandbox.path().join("a/b/c"))
            .assert()
            .success();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("node").eval(&output));
        assert!(predicate::str::contains("npm").eval(&output));
        assert!(predicate::str::contains("bun").eval(&output));
        assert!(predicate::str::contains("go").eval(&output));
    }

    #[test]
    fn global_doesnt_overwrite_local() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".proto/.prototools", r#"node = "18""#);
        sandbox.create_file("a/.prototools", r#"node = "20""#);

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("status")
            .arg("--include-global")
            .current_dir(sandbox.path().join("a"))
            .assert()
            .success();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("node").eval(&output));
        assert!(predicate::str::contains("20").eval(&output));
    }
}
