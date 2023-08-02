use std::path::PathBuf;
use warpgate::{GitHubLocator, PluginLocator, WapmLocator};

mod locator {
    use super::*;

    #[test]
    fn displays_correctly() {
        assert_eq!(
            PluginLocator::SourceFile {
                file: "foo.wasm".into(),
                path: PathBuf::from("/abs/foo.wasm"),
            }
            .to_string(),
            "source:foo.wasm"
        );

        assert_eq!(
            PluginLocator::SourceUrl {
                url: "https://download.com/bar.wasm".into()
            }
            .to_string(),
            "source:https://download.com/bar.wasm"
        );

        assert_eq!(
            PluginLocator::GitHub(GitHubLocator {
                file_prefix: "proto_plugin".into(),
                repo_slug: "moonrepo/proto".into(),
                tag: None,
            })
            .to_string(),
            "github:moonrepo/proto"
        );

        assert_eq!(
            PluginLocator::GitHub(GitHubLocator {
                file_prefix: "proto_plugin".into(),
                repo_slug: "moonrepo/proto".into(),
                tag: Some("latest".into()),
            })
            .to_string(),
            "github:moonrepo/proto@latest"
        );

        assert_eq!(
            PluginLocator::Wapm(WapmLocator {
                file_stem: "proto_plugin".into(),
                package_name: "moonrepo/proto".into(),
                version: None,
            })
            .to_string(),
            "wapm:moonrepo/proto"
        );

        assert_eq!(
            PluginLocator::Wapm(WapmLocator {
                file_stem: "proto_plugin".into(),
                package_name: "moonrepo/proto".into(),
                version: Some("1.2.3".into()),
            })
            .to_string(),
            "wapm:moonrepo/proto@1.2.3"
        );
    }

    #[test]
    #[should_panic(expected = "Missing plugin scope or location.")]
    fn errors_missing_scope() {
        PluginLocator::try_from("".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "Missing plugin location (after :).")]
    fn errors_missing_location() {
        PluginLocator::try_from("github:".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "Unknown plugin scope ``.")]
    fn errors_empty_scope() {
        PluginLocator::try_from(":foo.wasm".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "Unknown plugin scope `unknown`.")]
    fn errors_unknown_scope() {
        PluginLocator::try_from("unknown:foo.wasm".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "Missing plugin location (after :).")]
    fn errors_empty_location() {
        PluginLocator::try_from("source:".to_string()).unwrap();
    }

    mod source {
        use super::*;

        #[test]
        #[should_panic(expected = "Only https URLs are supported for source plugins.")]
        fn errors_http_source() {
            PluginLocator::try_from("source:http://domain.com/file.wasm".to_string()).unwrap();
        }

        #[test]
        fn parses_url() {
            assert_eq!(
                PluginLocator::try_from("source:https://domain.com/file.wasm".to_string()).unwrap(),
                PluginLocator::SourceUrl {
                    url: "https://domain.com/file.wasm".into()
                }
            );
        }

        #[test]
        fn parses_file() {
            assert_eq!(
                PluginLocator::try_from("source:file.wasm".to_string()).unwrap(),
                PluginLocator::SourceFile {
                    file: "file.wasm".into(),
                    path: PathBuf::from("file.wasm"),
                }
            );
        }

        #[test]
        fn parses_file_rel() {
            assert_eq!(
                PluginLocator::try_from("source:../file.wasm".to_string()).unwrap(),
                PluginLocator::SourceFile {
                    file: "../file.wasm".into(),
                    path: PathBuf::from("../file.wasm"),
                }
            );
            assert_eq!(
                PluginLocator::try_from("source:./file.wasm".to_string()).unwrap(),
                PluginLocator::SourceFile {
                    file: "./file.wasm".into(),
                    path: PathBuf::from("./file.wasm"),
                }
            );
        }
    }

    mod github {
        use super::*;

        #[test]
        #[should_panic(
            expected = "GitHub release locator requires a repository with organization scope (org/repo)."
        )]
        fn errors_no_slug() {
            PluginLocator::try_from("github:moonrepo".to_string()).unwrap();
        }

        #[test]
        fn parses_slug() {
            assert_eq!(
                PluginLocator::try_from("github:moonrepo/bun".to_string()).unwrap(),
                PluginLocator::GitHub(GitHubLocator {
                    file_prefix: "bun_plugin".into(),
                    repo_slug: "moonrepo/bun".into(),
                    tag: None,
                })
            );
        }

        #[test]
        fn parses_latest() {
            assert_eq!(
                PluginLocator::try_from("github:moonrepo/bun-plugin@latest".to_string()).unwrap(),
                PluginLocator::GitHub(GitHubLocator {
                    file_prefix: "bun_plugin".into(),
                    repo_slug: "moonrepo/bun-plugin".into(),
                    tag: Some("latest".into()),
                })
            );
        }

        #[test]
        fn parses_tag() {
            assert_eq!(
                PluginLocator::try_from("github:moonrepo/bun_plugin@v1.2.3".to_string()).unwrap(),
                PluginLocator::GitHub(GitHubLocator {
                    file_prefix: "bun_plugin".into(),
                    repo_slug: "moonrepo/bun_plugin".into(),
                    tag: Some("v1.2.3".into()),
                })
            );
        }
    }

    mod wapm {
        use super::*;

        #[test]
        #[should_panic(
            expected = "wapm.io locator requires a package with owner scope (owner/package)."
        )]
        fn errors_no_package() {
            PluginLocator::try_from("wapm:moonrepo".to_string()).unwrap();
        }

        #[test]
        fn parses_slug() {
            assert_eq!(
                PluginLocator::try_from("wapm:moonrepo/bun".to_string()).unwrap(),
                PluginLocator::Wapm(WapmLocator {
                    file_stem: "bun_plugin".into(),
                    package_name: "moonrepo/bun".into(),
                    version: None,
                })
            );
        }

        #[test]
        fn parses_latest() {
            assert_eq!(
                PluginLocator::try_from("wapm:moonrepo/bun-plugin@latest".to_string()).unwrap(),
                PluginLocator::Wapm(WapmLocator {
                    file_stem: "bun_plugin".into(),
                    package_name: "moonrepo/bun-plugin".into(),
                    version: Some("latest".into()),
                })
            );
        }

        #[test]
        fn parses_tag() {
            assert_eq!(
                PluginLocator::try_from("wapm:moonrepo/bun_plugin@1.2.3".to_string()).unwrap(),
                PluginLocator::Wapm(WapmLocator {
                    file_stem: "bun_plugin".into(),
                    package_name: "moonrepo/bun_plugin".into(),
                    version: Some("1.2.3".into()),
                })
            );
        }
    }
}
