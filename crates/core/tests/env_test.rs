use proto_core::{PinLocation, ProtoEnvironment};
use starbase_sandbox::create_empty_sandbox;
use std::path::Path;

fn create_env(sandbox_path: &Path, working_dir: &Path) -> ProtoEnvironment {
    let mut env = ProtoEnvironment::new_testing(sandbox_path).unwrap();
    env.working_dir = working_dir.to_path_buf();
    env
}

mod get_config_dir {
    use super::*;

    mod closest {
        use super::*;

        #[test]
        fn returns_working_dir_when_no_configs() {
            let sandbox = create_empty_sandbox();
            let cwd = sandbox.path().join("a/b/c");
            let env = create_env(sandbox.path(), &cwd);

            assert_eq!(env.get_config_dir(PinLocation::Closest).unwrap(), cwd);
        }

        #[test]
        fn returns_working_dir_when_it_has_a_config() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a/b/c/.prototools", "");

            let cwd = sandbox.path().join("a/b/c");
            let env = create_env(sandbox.path(), &cwd);

            assert_eq!(env.get_config_dir(PinLocation::Closest).unwrap(), cwd);
        }

        #[test]
        fn returns_parent_dir_with_a_config() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a/.prototools", "");

            let env = create_env(sandbox.path(), &sandbox.path().join("a/b/c"));

            assert_eq!(
                env.get_config_dir(PinLocation::Closest).unwrap(),
                sandbox.path().join("a")
            );
        }

        #[test]
        fn returns_closest_parent_dir_with_a_config() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");
            sandbox.create_file("a/b/.prototools", "");

            let env = create_env(sandbox.path(), &sandbox.path().join("a/b/c"));

            assert_eq!(
                env.get_config_dir(PinLocation::Closest).unwrap(),
                sandbox.path().join("a/b")
            );
        }

        #[test]
        fn prefers_working_dir_config_over_parents() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a/.prototools", "");
            sandbox.create_file("a/b/c/.prototools", "");

            let cwd = sandbox.path().join("a/b/c");
            let env = create_env(sandbox.path(), &cwd);

            assert_eq!(env.get_config_dir(PinLocation::Closest).unwrap(), cwd);
        }

        #[test]
        fn ignores_env_config_when_env_active() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a/.prototools.prod", "");

            let cwd = sandbox.path().join("a/b/c");
            let mut env = create_env(sandbox.path(), &cwd);
            env.env_mode = Some("prod".into());

            assert_eq!(env.get_config_dir(PinLocation::Closest).unwrap(), cwd);
        }

        #[test]
        fn returns_same_dir_regardless_of_env() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a/.prototools", "");
            sandbox.create_file("a/b/.prototools.prod", "");

            let cwd = sandbox.path().join("a/b/c");

            for env_mode in [None, Some("prod".to_owned())] {
                let mut env = create_env(sandbox.path(), &cwd);
                env.env_mode = env_mode;

                assert_eq!(
                    env.get_config_dir(PinLocation::Closest).unwrap(),
                    sandbox.path().join("a")
                );
            }
        }

        #[test]
        fn returns_dir_with_both_base_and_env_configs() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");
            sandbox.create_file("a/.prototools", "");
            sandbox.create_file("a/.prototools.prod", "");

            let mut env = create_env(sandbox.path(), &sandbox.path().join("a/b/c"));
            env.env_mode = Some("prod".into());

            assert_eq!(
                env.get_config_dir(PinLocation::Closest).unwrap(),
                sandbox.path().join("a")
            );
        }

        #[test]
        fn ignores_env_config_when_env_not_active() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a/.prototools.prod", "");

            let cwd = sandbox.path().join("a/b/c");
            let env = create_env(sandbox.path(), &cwd);

            assert_eq!(env.get_config_dir(PinLocation::Closest).unwrap(), cwd);
        }

        #[test]
        fn ignores_global_config() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".proto/.prototools", "");

            let cwd = sandbox.path().join("a/b/c");
            let env = create_env(sandbox.path(), &cwd);

            assert_eq!(env.get_config_dir(PinLocation::Closest).unwrap(), cwd);
        }

        #[test]
        fn ignores_user_config() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".home/.prototools", "");

            let cwd = sandbox.path().join(".home/projects/a");
            let env = create_env(sandbox.path(), &cwd);

            assert_eq!(env.get_config_dir(PinLocation::Closest).unwrap(), cwd);
        }

        #[test]
        fn returns_parent_dir_within_user_dir() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".home/.prototools", "");
            sandbox.create_file(".home/projects/.prototools", "");

            let env = create_env(sandbox.path(), &sandbox.path().join(".home/projects/a"));

            assert_eq!(
                env.get_config_dir(PinLocation::Closest).unwrap(),
                sandbox.path().join(".home/projects")
            );
        }
    }

    #[test]
    fn returns_static_dirs_for_other_locations() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/.prototools", "");

        let cwd = sandbox.path().join("a/b/c");
        let env = create_env(sandbox.path(), &cwd);

        assert_eq!(env.get_config_dir(PinLocation::Local).unwrap(), cwd);
        assert_eq!(
            env.get_config_dir(PinLocation::Global).unwrap(),
            sandbox.path().join(".proto")
        );
        assert_eq!(
            env.get_config_dir(PinLocation::User).unwrap(),
            sandbox.path().join(".home")
        );
    }
}

mod get_host_platform {
    use super::*;
    use system_env::{SystemArch, SystemLibc, SystemOS, SystemPlatform};

    #[test]
    fn detects_current_platform() {
        let sandbox = create_empty_sandbox();
        let env = create_env(sandbox.path(), sandbox.path());

        assert_eq!(env.get_host_platform(), &SystemPlatform::from_env());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn detects_libc_on_linux() {
        let sandbox = create_empty_sandbox();
        let env = create_env(sandbox.path(), sandbox.path());

        assert!(matches!(
            env.get_host_platform().libc,
            SystemLibc::Gnu | SystemLibc::Musl
        ));
    }

    #[test]
    fn can_override_platform() {
        let sandbox = create_empty_sandbox();
        let mut env = create_env(sandbox.path(), sandbox.path());
        let platform = SystemPlatform {
            os: SystemOS::Linux,
            arch: SystemArch::Riscv64,
            libc: SystemLibc::Musl,
        };

        env.set_host_platform(platform);

        assert_eq!(env.get_host_platform(), &platform);

        // Is preserved when cloning and reloading
        assert_eq!(env.clone().get_host_platform(), &platform);
        assert_eq!(env.reload().get_host_platform(), &platform);
    }

    #[test]
    fn overriding_doesnt_affect_clones() {
        let sandbox = create_empty_sandbox();
        let env = create_env(sandbox.path(), sandbox.path());
        let mut other = env.clone();

        other.set_host_platform(SystemPlatform::new(SystemOS::Solaris, SystemArch::Sparc64));

        assert_eq!(env.get_host_platform(), &SystemPlatform::from_env());
        assert_eq!(other.get_host_platform().os, SystemOS::Solaris);
    }
}

mod for_platform {
    use super::*;
    use system_env::{SystemArch, SystemLibc, SystemOS, SystemPlatform};

    fn foreign_platform() -> SystemPlatform {
        SystemPlatform::new(SystemOS::Linux, SystemArch::Riscv64).with_libc(SystemLibc::Musl)
    }

    #[test]
    fn targets_the_platform() {
        let sandbox = create_empty_sandbox();
        let env = create_env(sandbox.path(), sandbox.path());
        let other = env.for_platform(foreign_platform());

        assert_eq!(other.get_host_platform(), &foreign_platform());
        assert!(other.is_foreign_platform());

        // The original is not affected
        assert_eq!(env.get_host_platform(), &SystemPlatform::from_env());
        assert!(!env.is_foreign_platform());
    }

    #[test]
    fn is_not_foreign_when_targeting_the_host() {
        let sandbox = create_empty_sandbox();
        let env = create_env(sandbox.path(), sandbox.path());
        let other = env.for_platform(SystemPlatform::from_env());

        assert!(!other.is_foreign_platform());
    }

    #[test]
    fn remains_foreign_when_retargeting() {
        let sandbox = create_empty_sandbox();
        let env = create_env(sandbox.path(), sandbox.path());
        let other = env
            .for_platform(foreign_platform())
            .for_platform(foreign_platform());

        assert!(other.is_foreign_platform());
    }

    #[test]
    fn is_preserved_when_cloning_and_reloading() {
        let sandbox = create_empty_sandbox();
        let env = create_env(sandbox.path(), sandbox.path()).for_platform(foreign_platform());

        for other in [env.clone(), env.reload()] {
            assert_eq!(other.get_host_platform(), &foreign_platform());
            assert!(other.is_foreign_platform());
        }
    }

    #[test]
    fn shares_config_and_lockfile_state() {
        let sandbox = create_empty_sandbox();
        let env = create_env(sandbox.path(), sandbox.path());
        let other = env.for_platform(foreign_platform());

        assert!(std::ptr::eq(
            env.load_file_manager().unwrap(),
            other.load_file_manager().unwrap()
        ));
    }
}
