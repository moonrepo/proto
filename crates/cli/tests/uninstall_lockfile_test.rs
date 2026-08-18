use proto_core::test_utils::*;
use proto_core::{ProtoLock, UnresolvedVersionSpec};
use std::fs;
use std::path::Path;
use system_env::{SystemArch, SystemOS};

mod uninstall_lockfile {
    use super::*;

    fn inject_os_arch(sandbox: &Path) {
        let contents = fs::read_to_string(sandbox.join(".protolock")).unwrap();

        fs::write(
            sandbox.join(".protolock"),
            contents
                .replace("{os}", &SystemOS::default().to_string())
                .replace("{arch}", &SystemArch::default().to_string()),
        )
        .unwrap()
    }

    #[test]
    fn removes_matching_version_from_file() {
        let sandbox = create_proto_sandbox("lockfile-uninstall");
        inject_os_arch(sandbox.path());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("5.10.0");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall")
                    .arg("protostar")
                    .arg("5.10.0")
                    .arg("--yes");
            })
            .success();

        let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
        let records = lockfile.tools.get("protostar").unwrap();

        assert_eq!(records.len(), 1);
    }

    #[test]
    fn doesnt_remove_spec_from_file_even_if_versions_match() {
        let sandbox = create_proto_sandbox("lockfile-uninstall");
        inject_os_arch(sandbox.path());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("4.5.15");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall")
                    .arg("protostar")
                    .arg("4.5.15")
                    .arg("--yes");
            })
            .success();

        let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
        let records = lockfile.tools.get("protostar").unwrap();

        assert_eq!(records.len(), 2);
    }

    fn create_prune_sandbox() -> ProtoSandbox {
        let sandbox = create_empty_proto_sandbox();

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

        // Install first, so that the uninstall has something to remove
        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("5.10.0");
            })
            .success();

        // Then reseed the lockfile with stale records, as the install
        // flow itself will have already pruned orphans
        sandbox.create_file(
            ".protolock",
            format!(
                r#"
[[tools.protoform]]
os = "{os}"
arch = "{arch}"
spec = "3.0.0"
version = "3.0.0"

[[tools.moonstone]]
os = "{os}"
arch = "{arch}"
spec = "5.0.0"
version = "5.0.0"

[[tools.protostar]]
os = "{os}"
arch = "{arch}"
spec = "1"
version = "1.10.15"

[[tools.protostar]]
os = "{os}"
arch = "{arch}"
spec = "5.10.0"
version = "5.10.0"
"#,
                os = SystemOS::default(),
                arch = SystemArch::default()
            ),
        );

        sandbox
    }

    #[test]
    fn prunes_orphaned_records() {
        let sandbox = create_prune_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall")
                    .arg("protostar")
                    .arg("5.10.0")
                    .arg("--yes");
            })
            .success();

        let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();

        // The configured spec was kept, while the uninstalled and
        // stale specs were removed
        let protostar = lockfile.tools.get("protostar").unwrap();

        assert_eq!(protostar.len(), 1);
        assert_eq!(
            protostar[0].spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse("1").unwrap()
        );

        // Stale records for other configured tools are also pruned
        assert!(!lockfile.tools.contains_key("protoform"));

        // Unconfigured tools are ad-hoc installs and are kept
        assert_eq!(lockfile.tools.get("moonstone").unwrap().len(), 1);
    }

    #[test]
    fn prunes_orphaned_records_when_uninstalling_all() {
        let sandbox = create_prune_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("protostar").arg("--yes");
            })
            .success();

        let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();

        // All records for the uninstalled tool were removed
        assert!(!lockfile.tools.contains_key("protostar"));

        // Stale records for other configured tools are also pruned
        assert!(!lockfile.tools.contains_key("protoform"));

        // Unconfigured tools are ad-hoc installs and are kept
        assert_eq!(lockfile.tools.get("moonstone").unwrap().len(), 1);
    }

    #[test]
    fn deletes_file_if_no_contents() {
        let sandbox = create_proto_sandbox("lockfile");
        let lockfile_path = sandbox.path().join(".protolock");

        assert!(!lockfile_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("5.10.0");
            })
            .success();

        assert!(lockfile_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall")
                    .arg("protostar")
                    .arg("5.10.0")
                    .arg("--yes");
            })
            .success();

        assert!(!lockfile_path.exists());
    }

    mod env_mode {
        use super::*;

        fn create_env_sandbox() -> ProtoSandbox {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".prototools",
                r#"
protostar = "5.0.0"

[settings]
unstable-lockfile = true
"#,
            );
            sandbox.create_file(".prototools.production", r#"protostar = "5.10.0""#);

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar");
                })
                .success();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .env("PROTO_ENV", "production");
                })
                .success();

            assert!(sandbox.path().join(".protolock").exists());
            assert!(sandbox.path().join(".protolock.production").exists());

            sandbox
        }

        #[test]
        fn removes_version_from_env_lockfile_and_unpins_from_env_config() {
            let sandbox = create_env_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("uninstall")
                        .arg("protostar")
                        .arg("5.10.0")
                        .arg("--yes")
                        .env("PROTO_ENV", "production");
                })
                .success();

            // The env lockfile is now empty and removed
            assert!(!sandbox.path().join(".protolock.production").exists());

            // While the base lockfile is untouched
            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);

            // And the version is only unpinned from the env config
            let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

            assert!(config.contains("protostar"));

            let config = fs::read_to_string(sandbox.path().join(".prototools.production")).unwrap();

            assert!(!config.contains("protostar"));
        }

        #[test]
        fn removes_version_from_base_lockfile_when_env_config_takes_precedence() {
            let sandbox = create_env_sandbox();

            // The env config pins 5.10.0, but the base config pins 5.0.0
            sandbox
                .run_bin(|cmd| {
                    cmd.arg("uninstall")
                        .arg("protostar")
                        .arg("5.0.0")
                        .arg("--yes")
                        .env("PROTO_ENV", "production");
                })
                .success();

            // The base lockfile is now empty and removed
            assert!(!sandbox.path().join(".protolock").exists());

            // While the env lockfile is untouched
            let lockfile = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);

            // And the version is only unpinned from the base config
            let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

            assert!(!config.contains("protostar"));

            let config = fs::read_to_string(sandbox.path().join(".prototools.production")).unwrap();

            assert!(config.contains("protostar"));
        }

        #[test]
        fn removes_all_versions_from_all_lockfiles_and_unpins_from_all_configs() {
            let sandbox = create_env_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("uninstall")
                        .arg("protostar")
                        .arg("--yes")
                        .env("PROTO_ENV", "production");
                })
                .success();

            // Both lockfiles are now empty and removed
            assert!(!sandbox.path().join(".protolock").exists());
            assert!(!sandbox.path().join(".protolock.production").exists());

            // And the version is unpinned from both configs
            let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

            assert!(!config.contains("protostar"));

            let config = fs::read_to_string(sandbox.path().join(".prototools.production")).unwrap();

            assert!(!config.contains("protostar"));
        }
    }
}
