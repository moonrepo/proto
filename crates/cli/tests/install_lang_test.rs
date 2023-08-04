mod utils;

use proto_core::{ToolManifest, Version};
use rustc_hash::FxHashSet;
use starbase_sandbox::predicates::prelude::*;
use utils::*;

#[cfg(not(windows))]
mod go {
    use super::*;
    use std::fs;

    #[test]
    fn sets_gobin_to_shell() {
        let temp = create_empty_sandbox();
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
        let temp = create_empty_sandbox();
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
    use proto_core::AliasOrVersion;

    use super::*;

    #[test]
    fn installs_bundled_npm() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("node").arg("19.0.0").assert();

        let output = output_to_string(&assert.get_output().stderr.to_vec());

        assert!(predicate::str::contains("Node.js has been installed at").eval(&output));
        assert!(predicate::str::contains("npm has been installed at").eval(&output));

        assert!(temp.path().join("tools/node/19.0.0").exists());
        assert!(temp.path().join("tools/npm/8.19.2").exists());

        let manifest = ToolManifest::load(temp.path().join("tools/npm/manifest.json")).unwrap();

        assert_eq!(
            manifest.default_version,
            Some(AliasOrVersion::parse("8.19.2").unwrap())
        );
        assert_eq!(
            manifest.installed_versions,
            FxHashSet::from_iter([Version::parse("8.19.2").unwrap()])
        );
    }

    #[test]
    fn skips_bundled_npm() {
        let temp = create_empty_sandbox();

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

        assert!(temp.path().join("tools/node/19.0.0").exists());
        assert!(!temp.path().join("tools/npm/8.19.2").exists());
    }
}
