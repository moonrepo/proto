mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod plugin_search {
    use super::*;

    #[test]
    fn errors_if_no_query() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("plugin").arg("search").assert();

        assert.failure().stderr(predicate::str::contains(
            "the following required arguments were not provided",
        ));
    }

    #[test]
    fn errors_if_no_results() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("plugin").arg("search").arg("gibberish").assert();

        assert.failure().stderr(predicate::str::contains(
            "No plugins available for query \"gibberish\"",
        ));
    }

    #[test]
    fn returns_matching_results() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("plugin").arg("search").arg("zig").assert();

        assert
            .success()
            .stdout(predicate::str::contains("Available for query: zig"));
    }

    #[test]
    fn returns_json_data() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("plugin")
            .arg("search")
            .arg("zig")
            .arg("--json")
            .assert();

        assert.success().stdout(predicate::str::starts_with("["));
    }

    #[test]
    fn caches_results_locally() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("plugin").arg("search").arg("zig").assert();

        assert!(sandbox
            .path()
            .join(".proto/temp/registry-external-plugins.json")
            .exists());
    }
}
