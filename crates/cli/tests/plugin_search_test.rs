mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod plugin_search {
    use super::*;

    #[test]
    fn errors_if_no_query() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("plugin").arg("search");
            })
            .failure();

        assert.stderr(predicate::str::contains(
            "the following required arguments were not provided",
        ));
    }

    #[test]
    fn errors_if_no_results() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("plugin").arg("search").arg("gibberish");
            })
            .failure();

        assert.stderr(predicate::str::contains(
            "No plugins available for query \"gibberish\"",
        ));
    }

    #[test]
    fn returns_matching_results() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("plugin").arg("search").arg("zig");
            })
            .success();

        assert.stdout(predicate::str::contains("Available for query: zig"));
    }

    #[test]
    fn returns_json_data() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("plugin").arg("search").arg("zig").arg("--json");
            })
            .success();

        assert.stdout(predicate::str::starts_with("["));
    }

    #[test]
    fn caches_results_locally() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("plugin").arg("search").arg("zig");
            })
            .success();

        assert!(sandbox
            .path()
            .join(".proto/temp/registry-external-plugins.json")
            .exists());
    }
}
