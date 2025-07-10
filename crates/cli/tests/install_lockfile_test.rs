mod utils;

use proto_core::ProtoLock;
use proto_core::{
    Backend, Id, LockRecord, PinLocation, ProtoConfig, ToolManifest, ToolSpec,
    UnresolvedVersionSpec, VersionSpec,
};
use proto_pdk_api::Checksum;
use starbase_sandbox::predicates::prelude::*;
use std::collections::BTreeSet;
use std::fs;
use std::time::SystemTime;
use utils::*;

mod install_lockfile {
    use super::*;

    #[test]
    fn creates_lockfile() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
[settings]
unstable-lockfile = true
"#,
        );

        let lockfile_path = sandbox.path().join(".protolock");

        assert!(!lockfile_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("18.12.0");
            })
            .success();

        assert!(lockfile_path.exists());

        let lockfile = ProtoLock::load(lockfile_path).unwrap();

        let record = lockfile.tools.get("node").unwrap().iter().find(|rec| {
            rec.version
                .as_ref()
                .is_some_and(|version| version.to_string() == "18.12.0")
        });

        assert!(record.is_some());
    }
}
