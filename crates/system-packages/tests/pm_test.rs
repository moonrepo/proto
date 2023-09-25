use system_packages::*;

fn one_dep() -> DependencyConfig {
    SystemDependency::name("foo").to_config()
}

fn many_dep() -> DependencyConfig {
    SystemDependency::names(["foo", "bar", "baz"]).to_config()
}

mod pm {
    use super::*;

    mod apk {
        use super::*;

        #[test]
        fn install_package() {
            let pm = PackageClient::from(SystemPackageManager::Apk);
            let one_cfg = one_dep();
            let many_cfg = many_dep();

            assert_eq!(
                pm.get_install_command(&one_cfg, false).unwrap(),
                vec!["apk", "add", "foo"]
            );
            assert_eq!(
                pm.get_install_command(&one_cfg, true).unwrap(),
                vec!["apk", "add", "foo", "-i"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, false).unwrap(),
                vec!["apk", "add", "foo", "bar", "baz"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, true).unwrap(),
                vec!["apk", "add", "foo", "bar", "baz", "-i"]
            );
        }

        #[test]
        fn install_package_with_version() {
            let pm = PackageClient::from(SystemPackageManager::Apk);
            let mut cfg = one_dep();
            cfg.version = Some("1.2.3".into());

            assert_eq!(
                pm.get_install_command(&cfg, false).unwrap(),
                vec!["apk", "add", "foo=1.2.3"]
            );
        }

        #[test]
        fn update_index() {
            let pm = PackageClient::from(SystemPackageManager::Apk);

            assert_eq!(
                pm.get_update_index_command(false).unwrap(),
                vec!["apk", "update"]
            );
            assert_eq!(
                pm.get_update_index_command(true).unwrap(),
                vec!["apk", "update", "-i"]
            );
        }
    }

    mod apt {
        use super::*;

        #[test]
        fn install_package() {
            let pm = PackageClient::from(SystemPackageManager::Apt);
            let one_cfg = one_dep();
            let many_cfg = many_dep();

            assert_eq!(
                pm.get_install_command(&one_cfg, false).unwrap(),
                vec!["apt", "install", "--install-recommends", "foo", "-y"]
            );
            assert_eq!(
                pm.get_install_command(&one_cfg, true).unwrap(),
                vec!["apt", "install", "--install-recommends", "foo"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, false).unwrap(),
                vec![
                    "apt",
                    "install",
                    "--install-recommends",
                    "foo",
                    "bar",
                    "baz",
                    "-y"
                ]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, true).unwrap(),
                vec![
                    "apt",
                    "install",
                    "--install-recommends",
                    "foo",
                    "bar",
                    "baz"
                ]
            );
        }

        #[test]
        fn install_package_with_version() {
            let pm = PackageClient::from(SystemPackageManager::Apt);
            let mut cfg = one_dep();
            cfg.version = Some("1.2.3".into());

            assert_eq!(
                pm.get_install_command(&cfg, false).unwrap(),
                vec!["apt", "install", "--install-recommends", "foo=1.2.3", "-y"]
            );
        }

        #[test]
        fn update_index() {
            let pm = PackageClient::from(SystemPackageManager::Apt);

            assert_eq!(
                pm.get_update_index_command(false).unwrap(),
                vec!["apt", "update", "-y"]
            );
            assert_eq!(
                pm.get_update_index_command(true).unwrap(),
                vec!["apt", "update"]
            );
        }
    }

    mod brew {
        use super::*;

        #[test]
        fn install_package() {
            let pm = PackageClient::from(SystemPackageManager::Brew);
            let one_cfg = one_dep();
            let many_cfg = many_dep();

            assert_eq!(
                pm.get_install_command(&one_cfg, false).unwrap(),
                vec!["brew", "install", "foo"]
            );
            assert_eq!(
                pm.get_install_command(&one_cfg, true).unwrap(),
                vec!["brew", "install", "foo", "-i"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, false).unwrap(),
                vec!["brew", "install", "foo", "bar", "baz"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, true).unwrap(),
                vec!["brew", "install", "foo", "bar", "baz", "-i"]
            );
        }

        #[test]
        fn install_package_with_version() {
            let pm = PackageClient::from(SystemPackageManager::Brew);
            let mut cfg = one_dep();
            cfg.version = Some("1.2.3".into());

            assert_eq!(
                pm.get_install_command(&cfg, false).unwrap(),
                vec!["brew", "install", "foo@1.2.3"]
            );
        }

        #[test]
        fn update_index() {
            let pm = PackageClient::from(SystemPackageManager::Brew);

            assert_eq!(
                pm.get_update_index_command(false).unwrap(),
                vec!["brew", "update"]
            );
            assert_eq!(
                pm.get_update_index_command(true).unwrap(),
                vec!["brew", "update"]
            );
        }
    }

    mod choco {
        use super::*;

        #[test]
        fn install_package() {
            let pm = PackageClient::from(SystemPackageManager::Choco);
            let one_cfg = one_dep();
            let many_cfg = many_dep();

            assert_eq!(
                pm.get_install_command(&one_cfg, false).unwrap(),
                vec!["choco", "install", "foo", "-y"]
            );
            assert_eq!(
                pm.get_install_command(&one_cfg, true).unwrap(),
                vec!["choco", "install", "foo"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, false).unwrap(),
                vec!["choco", "install", "foo", "bar", "baz", "-y"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, true).unwrap(),
                vec!["choco", "install", "foo", "bar", "baz"]
            );
        }

        #[test]
        fn install_package_with_version() {
            let pm = PackageClient::from(SystemPackageManager::Choco);
            let mut cfg = one_dep();
            cfg.version = Some("1.2.3".into());

            assert_eq!(
                pm.get_install_command(&cfg, false).unwrap(),
                vec!["choco", "install", "foo", "--version", "1.2.3", "-y"]
            );
        }

        #[test]
        fn update_index() {
            let pm = PackageClient::from(SystemPackageManager::Choco);

            assert_eq!(pm.get_update_index_command(false), None);
        }
    }

    mod dnf {
        use super::*;

        #[test]
        fn install_package() {
            let pm = PackageClient::from(SystemPackageManager::Dnf);
            let one_cfg = one_dep();
            let many_cfg = many_dep();

            assert_eq!(
                pm.get_install_command(&one_cfg, false).unwrap(),
                vec!["dnf", "install", "foo", "-y"]
            );
            assert_eq!(
                pm.get_install_command(&one_cfg, true).unwrap(),
                vec!["dnf", "install", "foo"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, false).unwrap(),
                vec!["dnf", "install", "foo", "bar", "baz", "-y"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, true).unwrap(),
                vec!["dnf", "install", "foo", "bar", "baz"]
            );
        }

        #[test]
        fn install_package_with_version() {
            let pm = PackageClient::from(SystemPackageManager::Dnf);
            let mut cfg = one_dep();
            cfg.version = Some("1.2.3".into());

            assert_eq!(
                pm.get_install_command(&cfg, false).unwrap(),
                vec!["dnf", "install", "foo-1.2.3", "-y"]
            );
        }

        #[test]
        fn update_index() {
            let pm = PackageClient::from(SystemPackageManager::Dnf);

            assert_eq!(
                pm.get_update_index_command(false).unwrap(),
                vec!["dnf", "check-update", "-y"]
            );
            assert_eq!(
                pm.get_update_index_command(true).unwrap(),
                vec!["dnf", "check-update"]
            );
        }
    }

    mod pacman {
        use super::*;

        #[test]
        fn install_package() {
            let pm = PackageClient::from(SystemPackageManager::Pacman);
            let one_cfg = one_dep();
            let many_cfg = many_dep();

            assert_eq!(
                pm.get_install_command(&one_cfg, false).unwrap(),
                vec!["pacman", "-S", "foo", "--noconfirm"]
            );
            assert_eq!(
                pm.get_install_command(&one_cfg, true).unwrap(),
                vec!["pacman", "-S", "foo"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, false).unwrap(),
                vec!["pacman", "-S", "foo", "bar", "baz", "--noconfirm"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, true).unwrap(),
                vec!["pacman", "-S", "foo", "bar", "baz"]
            );
        }

        #[test]
        fn install_package_with_version() {
            let pm = PackageClient::from(SystemPackageManager::Pacman);
            let mut cfg = one_dep();
            cfg.version = Some("1.2.3".into());

            assert_eq!(
                pm.get_install_command(&cfg, false).unwrap(),
                vec!["pacman", "-S", "foo>=1.2.3", "--noconfirm"]
            );
        }

        #[test]
        fn update_index() {
            let pm = PackageClient::from(SystemPackageManager::Pacman);

            assert_eq!(
                pm.get_update_index_command(false).unwrap(),
                vec!["pacman", "-Syy"]
            );
            assert_eq!(
                pm.get_update_index_command(true).unwrap(),
                vec!["pacman", "-Syy"]
            );
        }
    }

    mod pkg {
        use super::*;

        #[test]
        fn install_package() {
            let pm = PackageClient::from(SystemPackageManager::Pkg);
            let one_cfg = one_dep();
            let many_cfg = many_dep();

            assert_eq!(
                pm.get_install_command(&one_cfg, false).unwrap(),
                vec!["pkg", "install", "foo", "-y"]
            );
            assert_eq!(
                pm.get_install_command(&one_cfg, true).unwrap(),
                vec!["pkg", "install", "foo"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, false).unwrap(),
                vec!["pkg", "install", "foo", "bar", "baz", "-y"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, true).unwrap(),
                vec!["pkg", "install", "foo", "bar", "baz"]
            );
        }

        #[test]
        fn install_package_with_version() {
            let pm = PackageClient::from(SystemPackageManager::Pkg);
            let mut cfg = one_dep();
            cfg.version = Some("1.2.3".into());

            assert_eq!(
                pm.get_install_command(&cfg, false).unwrap(),
                vec!["pkg", "install", "foo", "-y"]
            );
        }

        #[test]
        fn update_index() {
            let pm = PackageClient::from(SystemPackageManager::Pkg);

            assert_eq!(
                pm.get_update_index_command(false).unwrap(),
                vec!["pkg", "update"]
            );
            assert_eq!(
                pm.get_update_index_command(true).unwrap(),
                vec!["pkg", "update"]
            );
        }
    }

    mod pkgin {
        use super::*;

        #[test]
        fn install_package() {
            let pm = PackageClient::from(SystemPackageManager::Pkgin);
            let one_cfg = one_dep();
            let many_cfg = many_dep();

            assert_eq!(
                pm.get_install_command(&one_cfg, false).unwrap(),
                vec!["pkgin", "install", "foo", "-y"]
            );
            assert_eq!(
                pm.get_install_command(&one_cfg, true).unwrap(),
                vec!["pkgin", "install", "foo"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, false).unwrap(),
                vec!["pkgin", "install", "foo", "bar", "baz", "-y"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, true).unwrap(),
                vec!["pkgin", "install", "foo", "bar", "baz"]
            );
        }

        #[test]
        fn install_package_with_version() {
            let pm = PackageClient::from(SystemPackageManager::Pkgin);
            let mut cfg = one_dep();
            cfg.version = Some("1.2.3".into());

            assert_eq!(
                pm.get_install_command(&cfg, false).unwrap(),
                vec!["pkgin", "install", "foo-1.2.3", "-y"]
            );
        }

        #[test]
        fn update_index() {
            let pm = PackageClient::from(SystemPackageManager::Pkgin);

            assert_eq!(
                pm.get_update_index_command(false).unwrap(),
                vec!["pkgin", "update", "-y"]
            );
            assert_eq!(
                pm.get_update_index_command(true).unwrap(),
                vec!["pkgin", "update"]
            );
        }
    }

    mod scoop {
        use super::*;

        #[test]
        fn install_package() {
            let pm = PackageClient::from(SystemPackageManager::Scoop);
            let one_cfg = one_dep();
            let many_cfg = many_dep();

            assert_eq!(
                pm.get_install_command(&one_cfg, false).unwrap(),
                vec!["scoop", "install", "foo"]
            );
            assert_eq!(
                pm.get_install_command(&one_cfg, true).unwrap(),
                vec!["scoop", "install", "foo"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, false).unwrap(),
                vec!["scoop", "install", "foo", "bar", "baz"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, true).unwrap(),
                vec!["scoop", "install", "foo", "bar", "baz"]
            );
        }

        #[test]
        fn install_package_with_version() {
            let pm = PackageClient::from(SystemPackageManager::Scoop);
            let mut cfg = one_dep();
            cfg.version = Some("1.2.3".into());

            assert_eq!(
                pm.get_install_command(&cfg, false).unwrap(),
                vec!["scoop", "install", "foo@1.2.3"]
            );
        }

        #[test]
        fn update_index() {
            let pm = PackageClient::from(SystemPackageManager::Scoop);

            assert_eq!(
                pm.get_update_index_command(false).unwrap(),
                vec!["scoop", "update"]
            );
            assert_eq!(
                pm.get_update_index_command(true).unwrap(),
                vec!["scoop", "update"]
            );
        }
    }

    mod yum {
        use super::*;

        #[test]
        fn install_package() {
            let pm = PackageClient::from(SystemPackageManager::Yum);
            let one_cfg = one_dep();
            let many_cfg = many_dep();

            assert_eq!(
                pm.get_install_command(&one_cfg, false).unwrap(),
                vec!["yum", "install", "foo", "-y"]
            );
            assert_eq!(
                pm.get_install_command(&one_cfg, true).unwrap(),
                vec!["yum", "install", "foo"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, false).unwrap(),
                vec!["yum", "install", "foo", "bar", "baz", "-y"]
            );
            assert_eq!(
                pm.get_install_command(&many_cfg, true).unwrap(),
                vec!["yum", "install", "foo", "bar", "baz"]
            );
        }

        #[test]
        fn install_package_with_version() {
            let pm = PackageClient::from(SystemPackageManager::Yum);
            let mut cfg = one_dep();
            cfg.version = Some("1.2.3".into());

            assert_eq!(
                pm.get_install_command(&cfg, false).unwrap(),
                vec!["yum", "install", "foo-1.2.3", "-y"]
            );
        }

        #[test]
        fn update_index() {
            let pm = PackageClient::from(SystemPackageManager::Yum);

            assert_eq!(
                pm.get_update_index_command(false).unwrap(),
                vec!["yum", "check-update"]
            );
            assert_eq!(
                pm.get_update_index_command(true).unwrap(),
                vec!["yum", "check-update"]
            );
        }
    }
}
