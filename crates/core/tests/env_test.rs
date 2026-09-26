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
        fn returns_parent_dir_with_only_an_env_config() {
            let sandbox = create_empty_sandbox();
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
