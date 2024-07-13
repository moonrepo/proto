mod utils;

use starbase_sandbox::output_to_string;
use utils::*;

mod list_remote {
    use super::*;

    #[test]
    fn lists_remote_versions() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("list-remote").arg("npm");
        });

        // Without stderr
        let output = output_to_string(&assert.inner.get_output().stdout);

        assert!(output.split('\n').collect::<Vec<_>>().len() > 1);
    }
}
