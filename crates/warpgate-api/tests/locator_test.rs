use std::path::PathBuf;
use warpgate_api::{FileLocator, GitHubLocator, PluginLocator, UrlLocator};

mod locator {
    use super::*;

    #[test]
    fn displays_correctly() {
        assert_eq!(
            PluginLocator::File(Box::new(FileLocator {
                file: "foo.wasm".into(),
                path: Some(PathBuf::from("/abs/foo.wasm")),
            }))
            .to_string(),
            "file://foo.wasm"
        );

        assert_eq!(
            PluginLocator::Url(Box::new(UrlLocator {
                url: "https://download.com/bar.wasm".into()
            }))
            .to_string(),
            "https://download.com/bar.wasm"
        );

        assert_eq!(
            PluginLocator::GitHub(Box::new(GitHubLocator {
                repo_slug: "moonrepo/proto".into(),
                tag: None,
                project_name: None,
            }))
            .to_string(),
            "github://moonrepo/proto"
        );

        assert_eq!(
            PluginLocator::GitHub(Box::new(GitHubLocator {
                repo_slug: "moonrepo/proto".into(),
                tag: None,
                project_name: Some("tool".into()),
            }))
            .to_string(),
            "github://moonrepo/proto/tool"
        );

        assert_eq!(
            PluginLocator::GitHub(Box::new(GitHubLocator {
                repo_slug: "moonrepo/proto".into(),
                tag: Some("latest".into()),
                project_name: None,
            }))
            .to_string(),
            "github://moonrepo/proto@latest"
        );

        assert_eq!(
            PluginLocator::GitHub(Box::new(GitHubLocator {
                repo_slug: "moonrepo/proto".into(),
                tag: Some("latest".into()),
                project_name: Some("tool".into()),
            }))
            .to_string(),
            "github://moonrepo/proto/tool@latest"
        );
    }

    #[test]
    #[should_panic(expected = "MissingProtocol")]
    fn errors_missing_protocol() {
        PluginLocator::try_from("".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "MissingLocation")]
    fn errors_missing_location() {
        PluginLocator::try_from("github://".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "UnknownProtocol(\"\")")]
    fn errors_empty_protocol() {
        PluginLocator::try_from("://foo.wasm".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "UnknownProtocol(\"unknown\")")]
    fn errors_unknown_protocol() {
        PluginLocator::try_from("unknown://foo.wasm".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "MissingLocation")]
    fn errors_empty_location() {
        PluginLocator::try_from("file://".to_string()).unwrap();
    }

    mod registry {
        use warpgate_api::RegistryLocator;

        use super::*;

        #[test]
        #[should_panic(expected = "MissingLocation")]
        fn error_no_image() {
            PluginLocator::try_from("registry://".to_string()).unwrap();
        }

        #[test]
        #[should_panic(expected = "MissingRegistryImage")]
        fn error_no_image_but_tag() {
            PluginLocator::try_from("registry://:v0.0.0".to_string()).unwrap();
        }

        #[test]
        fn parses_image() {
            assert_eq!(
                PluginLocator::try_from("registry://java".to_string()).unwrap(),
                PluginLocator::Registry(Box::new(RegistryLocator {
                    registry: None,
                    repo_slug: None,
                    tag: None,
                    image: "java".into(),
                }))
            );
        }

        #[test]
        fn parses_slug() {
            assert_eq!(
                PluginLocator::try_from("registry://moonrepo/java".to_string()).unwrap(),
                PluginLocator::Registry(Box::new(RegistryLocator {
                    registry: Some("moonrepo".into()),
                    repo_slug: None,
                    tag: None,
                    image: "java".into(),
                }))
            );
        }

        #[test]
        fn parses_deep_slug() {
            assert_eq!(
                PluginLocator::try_from(
                    "registry://moonrepo/org/namespace1/namspace2/java".to_string()
                )
                .unwrap(),
                PluginLocator::Registry(Box::new(RegistryLocator {
                    registry: Some("moonrepo".into()),
                    repo_slug: Some("org/namespace1/namspace2".into()),
                    tag: None,
                    image: "java".into(),
                }))
            );
        }

        #[test]
        fn parses_tag_data() {
            assert_eq!(
                PluginLocator::try_from(
                    "registry://moonrepo/org/namespace1/namspace2/java:something".to_string()
                )
                .unwrap(),
                PluginLocator::Registry(Box::new(RegistryLocator {
                    registry: Some("moonrepo".into()),
                    repo_slug: Some("org/namespace1/namspace2".into()),
                    tag: Some("something".into()),
                    image: "java".into(),
                }))
            );
        }
    }

    mod file {
        use super::*;

        #[test]
        fn parses_file() {
            assert_eq!(
                PluginLocator::try_from("file://file.wasm".to_string()).unwrap(),
                PluginLocator::File(Box::new(FileLocator {
                    file: "file://file.wasm".into(),
                    path: None,
                }))
            );
        }

        #[test]
        fn parses_file_legacy() {
            assert_eq!(
                PluginLocator::try_from("source:file.wasm".to_string()).unwrap(),
                PluginLocator::File(Box::new(FileLocator {
                    file: "file://file.wasm".into(),
                    path: None,
                }))
            );
        }

        #[test]
        fn parses_file_rel() {
            assert_eq!(
                PluginLocator::try_from("file://../file.wasm".to_string()).unwrap(),
                PluginLocator::File(Box::new(FileLocator {
                    file: "file://../file.wasm".into(),
                    path: None,
                }))
            );
            assert_eq!(
                PluginLocator::try_from("file://./file.wasm".to_string()).unwrap(),
                PluginLocator::File(Box::new(FileLocator {
                    file: "file://./file.wasm".into(),
                    path: None,
                }))
            );
        }

        #[test]
        fn parses_file_rel_legacy() {
            assert_eq!(
                PluginLocator::try_from("source:../file.wasm".to_string()).unwrap(),
                PluginLocator::File(Box::new(FileLocator {
                    file: "file://../file.wasm".into(),
                    path: None,
                }))
            );
            assert_eq!(
                PluginLocator::try_from("source:./file.wasm".to_string()).unwrap(),
                PluginLocator::File(Box::new(FileLocator {
                    file: "file://./file.wasm".into(),
                    path: None,
                }))
            );
        }
    }

    mod github {
        use super::*;

        #[test]
        #[should_panic(expected = "MissingGitHubOrg")]
        fn errors_no_slug() {
            PluginLocator::try_from("github://moonrepo".to_string()).unwrap();
        }

        #[test]
        fn parses_slug_legacy() {
            assert_eq!(
                PluginLocator::try_from("github:moonrepo/bun".to_string()).unwrap(),
                PluginLocator::GitHub(Box::new(GitHubLocator {
                    repo_slug: "moonrepo/bun".into(),
                    tag: None,
                    project_name: None,
                }))
            );
        }

        #[test]
        fn parses_slug() {
            assert_eq!(
                PluginLocator::try_from("github://moonrepo/bun".to_string()).unwrap(),
                PluginLocator::GitHub(Box::new(GitHubLocator {
                    repo_slug: "moonrepo/bun".into(),
                    tag: None,
                    project_name: None,
                }))
            );
        }

        #[test]
        fn parses_slug_with_file() {
            assert_eq!(
                PluginLocator::try_from("github://moonrepo/plugins/bun_tool".to_string()).unwrap(),
                PluginLocator::GitHub(Box::new(GitHubLocator {
                    repo_slug: "moonrepo/plugins".into(),
                    tag: None,
                    project_name: Some("bun_tool".into()),
                }))
            );
        }

        #[test]
        fn parses_latest() {
            assert_eq!(
                PluginLocator::try_from("github://moonrepo/bun-plugin@latest".to_string()).unwrap(),
                PluginLocator::GitHub(Box::new(GitHubLocator {
                    repo_slug: "moonrepo/bun-plugin".into(),
                    tag: Some("latest".into()),
                    project_name: None,
                }))
            );
        }

        #[test]
        fn parses_tag() {
            assert_eq!(
                PluginLocator::try_from("github://moonrepo/bun_plugin@v1.2.3".to_string()).unwrap(),
                PluginLocator::GitHub(Box::new(GitHubLocator {
                    repo_slug: "moonrepo/bun_plugin".into(),
                    tag: Some("v1.2.3".into()),
                    project_name: None,
                }))
            );
        }

        #[test]
        fn parses_tag_with_file() {
            assert_eq!(
                PluginLocator::try_from("github://moonrepo/plugins/bun_tool@v1.2.3".to_string())
                    .unwrap(),
                PluginLocator::GitHub(Box::new(GitHubLocator {
                    repo_slug: "moonrepo/plugins".into(),
                    tag: Some("v1.2.3".into()),
                    project_name: Some("bun_tool".into()),
                }))
            );
        }
    }

    mod url {
        use super::*;

        #[test]
        #[should_panic(expected = "SecureUrlsOnly")]
        fn errors_http_source() {
            PluginLocator::try_from("http://domain.com/file.wasm".to_string()).unwrap();
        }

        #[test]
        fn parses_url() {
            assert_eq!(
                PluginLocator::try_from("https://domain.com/file.wasm".to_string()).unwrap(),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://domain.com/file.wasm".into()
                }))
            );
        }

        #[test]
        fn parses_url_legacy() {
            assert_eq!(
                PluginLocator::try_from("source:https://domain.com/file.wasm".to_string()).unwrap(),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://domain.com/file.wasm".into()
                }))
            );
        }
    }
}
