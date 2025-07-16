mod utils;

use proto_core::{ProtoLock, UnresolvedVersionSpec, VersionSpec};
use proto_pdk_api::ChecksumAlgorithm;
use utils::*;

macro_rules! assert_record {
    ($var:expr, $spec:literal, $ver:literal, $checksum:literal) => {
        assert_eq!(
            $var.spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse($spec).unwrap()
        );
        assert_eq!(
            $var.version.as_ref().unwrap(),
            &VersionSpec::parse($ver).unwrap()
        );

        let checksum = $var.checksum.as_ref().unwrap();

        assert_eq!(checksum.algo, ChecksumAlgorithm::Sha256);
        assert_eq!(checksum.hash.as_ref().unwrap(), $checksum);
    };
}

mod install_all_lockfile {
    use super::*;

    #[test]
    fn adds_all_to_lockfile() {
        let sandbox = create_proto_sandbox("lockfile-all");
        let lockfile_path = sandbox.path().join(".protolock");

        assert!(!lockfile_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install");
            })
            .success();

        assert!(lockfile_path.exists());

        let lockfile = ProtoLock::load(lockfile_path).unwrap();

        let protostar = lockfile.tools.get("protostar").unwrap().first().unwrap();

        assert_record!(
            protostar,
            "1",
            "1.10.15",
            "602c5dc51581977f71cc2fb181099846a441598283ea74cba45eb1cf99f7548d"
        );

        let protoform = lockfile.tools.get("protoform").unwrap().first().unwrap();

        assert_record!(
            protoform,
            "2.1",
            "2.1.15",
            "882a417d8801e2aead2142f47c9f19bd48bf8ddc03c0ca15c150c58ddc9836c8"
        );

        let moonbase = lockfile.tools.get("moonbase").unwrap().first().unwrap();

        assert_record!(
            moonbase,
            "3.2.1",
            "3.2.1",
            "31af59c90c69fb80f10e2036bfac2b98cc98f5b12437211a496787e9561a131b"
        );

        let moonstone = lockfile.tools.get("moonstone").unwrap().first().unwrap();

        assert_record!(
            moonstone,
            "4.10",
            "4.10.15",
            "821c8166c5567859da2a1597c8a07b85776a6adf56ecaec4ad95de978fac31ba"
        );
    }
}
