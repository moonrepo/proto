mod utils;

use proto_core::{ToolManifest, VersionType};
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeMap;
use utils::*;

mod unalias {
    use super::*;

    #[test]
    fn errors_unknown_tool() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("unalias").arg("unknown").arg("alias").assert();

        assert.stderr(predicate::str::contains("unknown is not a built-in tool"));
    }

    #[test]
    fn removes_existing_alias() {
        let sandbox = create_empty_sandbox();
        let manifest_file = sandbox.path().join("tools/node/manifest.json");

        let mut manifest = ToolManifest::load(&manifest_file).unwrap();
        manifest
            .aliases
            .insert("example".into(), VersionType::parse("19.0.0").unwrap());
        manifest.save().unwrap();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("unalias")
            .arg("node")
            .arg("example")
            .assert()
            .success();

        let manifest = ToolManifest::load(&manifest_file).unwrap();

        assert!(manifest.aliases.is_empty());
    }

    #[test]
    fn does_nothing_for_unknown_alias() {
        let sandbox = create_empty_sandbox();
        let manifest_file = sandbox.path().join("tools/node/manifest.json");

        let mut manifest = ToolManifest::load(&manifest_file).unwrap();
        manifest
            .aliases
            .insert("example".into(), VersionType::parse("19.0.0").unwrap());
        manifest.save().unwrap();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("unalias")
            .arg("node")
            .arg("unknown")
            .assert()
            .success();

        let manifest = ToolManifest::load(manifest_file).unwrap();

        assert_eq!(
            manifest.aliases,
            BTreeMap::from_iter([("example".into(), VersionType::parse("19.0.0").unwrap())])
        );
    }
}
