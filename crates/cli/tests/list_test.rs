mod utils;

use proto_core::{ToolManifest, VersionSpec};
use starbase_sandbox::output_to_string;
use utils::*;

mod list {
    use super::*;

    #[test]
    fn lists_local_versions() {
        let sandbox = create_empty_proto_sandbox();

        let mut manifest =
            ToolManifest::load(sandbox.path().join(".proto/tools/node/manifest.json")).unwrap();
        manifest
            .installed_versions
            .insert(VersionSpec::parse("19.0.0").unwrap());
        manifest
            .installed_versions
            .insert(VersionSpec::parse("18.0.0").unwrap());
        manifest
            .installed_versions
            .insert(VersionSpec::parse("17.0.0").unwrap());
        manifest.save().unwrap();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("list").arg("node");
        });

        // Without stderr
        let output = output_to_string(&assert.inner.get_output().stdout);

        assert_eq!(output.split('\n').collect::<Vec<_>>().len(), 4); // includes header
    }
}
