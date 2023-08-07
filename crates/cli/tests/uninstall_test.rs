mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod uninstall {
    use super::*;

    #[test]
    fn doesnt_uninstall_tool_if_doesnt_exist() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("uninstall").arg("node").arg("19.0.0").assert();

        assert.stderr(predicate::str::contains("Node.js 19.0.0 does not exist!"));
    }
}
