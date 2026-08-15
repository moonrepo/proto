use proto_core::test_utils::*;
use proto_core::{ProtoLock, UnresolvedVersionSpec, VersionSpec};
use proto_pdk_api::ChecksumAlgorithm;
use system_env::{SystemArch, SystemOS};

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

    #[test]
    fn doesnt_add_tools_from_nested_configs() {
        let sandbox = create_proto_sandbox("lockfile-nested");
        let nested_dir = sandbox.path().join("nested");

        // The nested config isn't locked, so the parent lockfile
        // shouldn't include its tools
        sandbox.create_file("nested/.prototools", r#"moonstone = "1.2.0""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").current_dir(&nested_dir);
            })
            .success();

        let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();

        assert!(lockfile.tools.contains_key("protostar"));
        assert!(!lockfile.tools.contains_key("moonstone"));

        assert!(!nested_dir.join(".protolock").exists());
    }

    #[test]
    fn supports_lockfiles_in_nested_configs() {
        let sandbox = create_proto_sandbox("lockfile-nested");
        let nested_dir = sandbox.path().join("nested");

        sandbox.create_file(
            "nested/.prototools",
            r#"
moonstone = "1.2.0"

[settings]
unstable-lockfile = true
"#,
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").current_dir(&nested_dir);
            })
            .success();

        // Each lockfile only includes tools from its own config
        let root_lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();

        assert!(root_lockfile.tools.contains_key("protostar"));
        assert!(!root_lockfile.tools.contains_key("moonstone"));

        let nested_lockfile = ProtoLock::load(nested_dir.join(".protolock")).unwrap();

        assert!(!nested_lockfile.tools.contains_key("protostar"));

        let moonstone = nested_lockfile
            .tools
            .get("moonstone")
            .unwrap()
            .first()
            .unwrap();

        assert_eq!(
            moonstone.spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse("1.2.0").unwrap()
        );
        assert_eq!(
            moonstone.version.as_ref().unwrap(),
            &VersionSpec::parse("1.2.0").unwrap()
        );
    }

    #[test]
    fn locks_env_configs_to_their_own_lockfiles() {
        let sandbox = create_proto_sandbox("lockfile-nested");
        sandbox.create_file(
            ".prototools",
            r#"
protostar = "1"
protoform = "2.1"

[settings]
unstable-lockfile = true
builtin-backends = false
builtin-tools = false
"#,
        );
        sandbox.create_file(".prototools.production", r#"protostar = "5.0.0""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").env("PROTO_ENV", "production");
            })
            .success();

        // Tools defined in the env config are locked to the env lockfile
        let env_lockfile = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();

        assert!(!env_lockfile.tools.contains_key("protoform"));

        let protostar = env_lockfile.tools.get("protostar").unwrap();

        assert_eq!(protostar.len(), 1);
        assert_eq!(
            protostar[0].spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse("5.0.0").unwrap()
        );
        assert_eq!(
            protostar[0].version.as_ref().unwrap(),
            &VersionSpec::parse("5.0.0").unwrap()
        );
        assert!(protostar[0].checksum.is_some());

        // While tools defined in the base config are locked to the base lockfile,
        // which doesn't include the version overridden by the env config
        let base_lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();

        assert!(!base_lockfile.tools.contains_key("protostar"));

        let protoform = base_lockfile.tools.get("protoform").unwrap();

        assert_eq!(protoform.len(), 1);
        assert_record!(
            protoform[0],
            "2.1",
            "2.1.15",
            "882a417d8801e2aead2142f47c9f19bd48bf8ddc03c0ca15c150c58ddc9836c8"
        );

        // Installing without the env only touches the base lockfile
        sandbox
            .run_bin(|cmd| {
                cmd.arg("install");
            })
            .success();

        let base_lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
        let protostar = base_lockfile.tools.get("protostar").unwrap();

        assert_eq!(protostar.len(), 1);
        assert_record!(
            protostar[0],
            "1",
            "1.10.15",
            "602c5dc51581977f71cc2fb181099846a441598283ea74cba45eb1cf99f7548d"
        );

        let env_lockfile = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();

        assert_eq!(env_lockfile.tools.len(), 1);
        assert_eq!(env_lockfile.tools.get("protostar").unwrap().len(), 1);
    }
}
