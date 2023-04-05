mod utils;

use predicates::prelude::*;
use std::fs;
use utils::*;

#[cfg(not(windows))]
mod go {
    use super::*;

    #[test]
    fn sets_gobin_to_shell() {
        let temp = create_temp_dir();
        let profile = temp.path().join(".profile");

        let mut cmd = create_proto_command(temp.path());

        cmd.env("TEST_PROFILE", &profile)
            .arg("install")
            .arg("go")
            .arg("1.20.0")
            .assert()
            .success();

        let output = fs::read_to_string(profile).unwrap();

        assert!(predicate::str::contains("GOBIN=\"$HOME/go/bin\"").eval(&output));
    }

    #[test]
    fn doesnt_set_gobin() {
        let temp = create_temp_dir();
        let profile = temp.path().join(".profile");

        let mut cmd = create_proto_command(temp.path());

        cmd.env("TEST_PROFILE", &profile)
            .arg("install")
            .arg("go")
            .arg("1.20.0")
            .arg("--")
            .arg("--no-gobin")
            .assert()
            .success();

        assert!(!profile.exists());
    }
}

mod node {
    use super::*;

    #[test]
    fn installs_bundled_npm() {
        let temp = create_temp_dir();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("node").arg("19.0.0").assert();

        let output = output_to_string(&assert.get_output().stderr.to_vec());

        assert!(predicate::str::contains("Node.js has been installed at").eval(&output));
        assert!(predicate::str::contains("npm has been installed at").eval(&output));

        assert!(temp.join("tools/node/19.0.0").exists());
        assert!(temp.join("tools/npm/8.19.2").exists());

        assert_eq!(
            fs::read_to_string(temp.join("tools/npm/manifest.json")).unwrap(),
            r#"{
  "aliases": {},
  "default_version": "bundled",
  "installed_versions": [
    "8.19.2"
  ]
}"#
        );
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

        let output = output_to_string(&assert.get_output().stderr.to_vec());

        assert!(predicate::str::contains("Node.js has been installed at").eval(&output));
        assert!(!predicate::str::contains("npm has been installed at").eval(&output));

        assert!(temp.join("tools/node/19.0.0").exists());
        assert!(!temp.join("tools/npm/8.19.2").exists());
    }
}
