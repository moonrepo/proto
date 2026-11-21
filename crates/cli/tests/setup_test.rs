// Windows resolves the home directory without `HOME`, and also updates the
// system `PATH` during setup
#[cfg(unix)]
mod setup {
    use proto_core::test_utils::*;
    use starbase_sandbox::assert_cmd::Command;
    use starbase_sandbox::{Sandbox, SandboxAssert};
    use std::fs;
    use std::path::{Path, PathBuf};

    const PROTO_SETUP: &str = "set -gx PROTO_HOME \"$HOME/.proto\";\n";

    fn run_fish_setup(sandbox: &Sandbox, handler: impl FnOnce(&mut Command)) -> SandboxAssert<'_> {
        sandbox.run_bin(|cmd| {
            cmd.arg("setup")
                .arg("--shell")
                .arg("fish")
                .arg("--no-modify-path")
                .arg("--yes")
                .env("HOME", sandbox.path().join(".home"))
                // Otherwise setup is skipped, as proto is already on PATH
                .env("PATH", "/usr/bin:/bin")
                .env_remove("XDG_CONFIG_HOME")
                .env_remove("PROTO_SHELL_PROFILE")
                .env_remove("PROTO_NO_MODIFY_PROFILE");

            handler(cmd);
        })
    }

    fn fish_dir(sandbox: &Sandbox) -> PathBuf {
        sandbox.path().join(".home/.config/fish")
    }

    fn read_file(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    fn write_file(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn preferred_profile(sandbox: &Sandbox) -> PathBuf {
        read_file(&sandbox.path().join(".proto/profile")).into()
    }

    #[test]
    fn sets_up_config_fish_by_default() {
        let sandbox = create_empty_proto_sandbox();
        let config_path = fish_dir(&sandbox).join("config.fish");

        run_fish_setup(&sandbox, |_| {}).success();

        let config = read_file(&config_path);

        assert!(config.contains("set -gx PROTO_HOME"));
        assert!(config.contains("set -gx PATH \"$PROTO_HOME/shims\" \"$PROTO_HOME/bin\" $PATH;"));
        assert!(!fish_dir(&sandbox).join("conf.d/proto.fish").exists());
        assert_eq!(preferred_profile(&sandbox), config_path);
    }

    #[test]
    fn sets_up_missing_profile_from_env_var() {
        let sandbox = create_empty_proto_sandbox();
        let conf_path = fish_dir(&sandbox).join("conf.d/proto.fish");

        run_fish_setup(&sandbox, |cmd| {
            cmd.env("PROTO_SHELL_PROFILE", &conf_path);
        })
        .success();

        assert!(read_file(&conf_path).contains("set -gx PROTO_HOME"));
        assert!(!fish_dir(&sandbox).join("config.fish").exists());
        assert_eq!(preferred_profile(&sandbox), conf_path);
    }

    #[test]
    fn keeps_using_conf_d_when_already_set_up() {
        let sandbox = create_empty_proto_sandbox();
        let config_path = fish_dir(&sandbox).join("config.fish");
        let conf_path = fish_dir(&sandbox).join("conf.d/proto.fish");

        write_file(&config_path, "# mine\n");
        write_file(&conf_path, PROTO_SETUP);

        run_fish_setup(&sandbox, |_| {}).success();

        // Neither file is set up again
        assert_eq!(read_file(&config_path), "# mine\n");
        assert_eq!(read_file(&conf_path), PROTO_SETUP);
        assert_eq!(preferred_profile(&sandbox), conf_path);
    }

    #[test]
    fn keeps_using_config_fish_when_already_set_up() {
        let sandbox = create_empty_proto_sandbox();
        let config_path = fish_dir(&sandbox).join("config.fish");

        write_file(&config_path, PROTO_SETUP);

        run_fish_setup(&sandbox, |_| {}).success();

        assert_eq!(read_file(&config_path), PROTO_SETUP);
        assert!(!fish_dir(&sandbox).join("conf.d/proto.fish").exists());
        assert_eq!(preferred_profile(&sandbox), config_path);
    }

    #[test]
    fn finds_conf_d_in_xdg_config_home() {
        let sandbox = create_empty_proto_sandbox();
        let xdg_dir = sandbox.path().join(".xdg");
        let config_path = xdg_dir.join("fish/config.fish");
        let conf_path = xdg_dir.join("fish/conf.d/proto.fish");

        write_file(&config_path, "# mine\n");
        write_file(&conf_path, PROTO_SETUP);

        run_fish_setup(&sandbox, |cmd| {
            cmd.env("XDG_CONFIG_HOME", &xdg_dir);
        })
        .success();

        assert_eq!(read_file(&config_path), "# mine\n");
        assert_eq!(read_file(&conf_path), PROTO_SETUP);
        assert_eq!(preferred_profile(&sandbox), conf_path);
    }
}
