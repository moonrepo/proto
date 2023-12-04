mod utils;

use utils::*;

mod list_remote {
    use super::*;

    #[test]
    fn lists_remote_versions() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("list-remote").arg("npm").assert();

        let output = output_to_string(&assert.get_output().stdout);

        assert!(output.split('\n').collect::<Vec<_>>().len() > 1);
    }
}
