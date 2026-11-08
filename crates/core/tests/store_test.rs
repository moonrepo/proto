use proto_core::layout::Store;
use starbase_sandbox::create_empty_sandbox;
use std::fs;

mod check_bin_linking {
    use super::*;

    #[test]
    fn passes_and_leaves_nothing_behind() {
        let sandbox = create_empty_sandbox();
        let store = Store::new(sandbox.path());

        store.check_bin_linking().unwrap();

        assert_eq!(fs::read_dir(&store.bin_dir).unwrap().count(), 0);
    }

    #[test]
    fn errors_if_bin_dir_is_unusable() {
        let sandbox = create_empty_sandbox();
        let store = Store::new(sandbox.path());

        sandbox.create_file("bin", "");

        assert!(store.check_bin_linking().is_err());
    }
}

mod link_bin {
    use super::*;

    #[test]
    fn creates_a_symlink_or_skips_without_copying() {
        let sandbox = create_empty_sandbox();
        let store = Store::new(sandbox.path());
        let src_path = sandbox.path().join("tools/tool/1.0.0/tool");
        let bin_path = sandbox.path().join("bin/tool");

        sandbox.create_file("tools/tool/1.0.0/tool", "exe");
        fs::create_dir_all(&store.bin_dir).unwrap();

        let linked = store.link_bin(&bin_path, &src_path).unwrap();

        // Windows may not have access to create symlinks
        assert!(linked || cfg!(windows));
        assert_eq!(bin_path.is_symlink(), linked);
        assert_eq!(bin_path.exists(), linked);
    }

    #[cfg(windows)]
    #[test]
    fn skips_if_it_cant_link() {
        let sandbox = create_empty_sandbox();
        let store = Store::new(sandbox.path());
        let src_path = sandbox.path().join("tools/tool/1.0.0/tool.exe");
        let bin_path = sandbox.path().join("bin/tool.exe");

        sandbox.create_file("tools/tool/1.0.0/tool.exe", "exe");

        // The parent directory is missing, so the link fails
        assert!(!store.link_bin(&bin_path, &src_path).unwrap());
        assert!(!bin_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn errors_if_it_cant_link() {
        let sandbox = create_empty_sandbox();
        let store = Store::new(sandbox.path());
        let src_path = sandbox.path().join("tools/tool/1.0.0/tool");
        let bin_path = sandbox.path().join("bin/tool");

        sandbox.create_file("tools/tool/1.0.0/tool", "exe");

        assert!(store.link_bin(&bin_path, &src_path).is_err());
    }
}
