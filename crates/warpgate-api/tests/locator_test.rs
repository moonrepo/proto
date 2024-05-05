use std::path::PathBuf;
use warpgate_api::{GitHubLocator, PluginLocator};

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
            PluginLocator::GitHub(Box::new(GitHubLocator {
                file_prefix: "proto_plugin".into(),
                repo_slug: "moonrepo/proto".into(),
                tag: None,
            }))
            .to_string(),
            "github:moonrepo/proto"
        );

        assert_eq!(
            PluginLocator::GitHub(Box::new(GitHubLocator {
                file_prefix: "proto_plugin".into(),
                repo_slug: "moonrepo/proto".into(),
                tag: Some("latest".into()),
            }))
            .to_string(),
            "github:moonrepo/proto@latest"
        );
    }

    #[test]
    #[should_panic(expected = "MissingScope")]
    fn errors_missing_scope() {
        PluginLocator::try_from("".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "MissingLocation")]
    fn errors_missing_location() {
        PluginLocator::try_from("github:".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "UnknownScope(\"\")")]
    fn errors_empty_scope() {
        PluginLocator::try_from(":foo.wasm".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "UnknownScope(\"unknown\")")]
    fn errors_unknown_scope() {
        PluginLocator::try_from("unknown:foo.wasm".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "MissingLocation")]
    fn errors_empty_location() {
        PluginLocator::try_from("source:".to_string()).unwrap();
    }

    mod source {
        use super::*;

        #[test]
        #[should_panic(expected = "SecureUrlsOnly")]
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
        #[should_panic(expected = "GitHubMissingOrg")]
        fn errors_no_slug() {
            PluginLocator::try_from("github:moonrepo".to_string()).unwrap();
        }

        #[test]
        fn parses_slug() {
            assert_eq!(
                PluginLocator::try_from("github:moonrepo/bun".to_string()).unwrap(),
                PluginLocator::GitHub(Box::new(GitHubLocator {
                    file_prefix: "bun_plugin".into(),
                    repo_slug: "moonrepo/bun".into(),
                    tag: None,
                }))
            );
        }

        #[test]
        fn parses_latest() {
            assert_eq!(
                PluginLocator::try_from("github:moonrepo/bun-plugin@latest".to_string()).unwrap(),
                PluginLocator::GitHub(Box::new(GitHubLocator {
                    file_prefix: "bun_plugin".into(),
                    repo_slug: "moonrepo/bun-plugin".into(),
                    tag: Some("latest".into()),
                }))
            );
        }

        #[test]
        fn parses_tag() {
            assert_eq!(
                PluginLocator::try_from("github:moonrepo/bun_plugin@v1.2.3".to_string()).unwrap(),
                PluginLocator::GitHub(Box::new(GitHubLocator {
                    file_prefix: "bun_plugin".into(),
                    repo_slug: "moonrepo/bun_plugin".into(),
                    tag: Some("v1.2.3".into()),
                }))
            );
        }
    }
}
