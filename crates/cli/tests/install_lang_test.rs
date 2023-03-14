mod utils;

use predicates::prelude::*;
use utils::*;

mod node {
    use super::*;

    #[test]
    fn installs_bundled_npm() {
        let temp = create_temp_dir();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("node").arg("19.0.0").assert();

        assert!(temp.join("tools/node/19.0.0").exists());
        assert!(temp.join("tools/npm/8.19.2").exists());

        let output = output_to_string(&assert.get_output().stderr.to_vec());

        assert!(predicate::str::contains("Node.js has been installed at").eval(&output));
        assert!(predicate::str::contains("npm has been installed at").eval(&output));
    }

    #[test]
    fn skips_bundled_npm() {
        let temp = create_temp_dir();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        assert!(temp.join("tools/node/19.0.0").exists());
        assert!(!temp.join("tools/npm/8.19.2").exists());

        let output = output_to_string(&assert.get_output().stderr.to_vec());

        assert!(predicate::str::contains("Node.js has been installed at").eval(&output));
        assert!(!predicate::str::contains("npm has been installed at").eval(&output));
    }
}
