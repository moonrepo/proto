use proto_core::{Backend, ToolSpec, UnresolvedVersionSpec};

mod tool_spec {
    use super::*;

    #[test]
    #[should_panic(expected = "UnknownBackend")]
    fn errors_unknown_backend() {
        ToolSpec::parse("fake:123").unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidVersionSpec")]
    fn errors_invalid_spec() {
        ToolSpec::parse("asdf:1.a.2").unwrap();
    }

    #[test]
    fn parses_latest() {
        assert_eq!(
            ToolSpec::parse("latest").unwrap(),
            ToolSpec {
                backend: None,
                req: UnresolvedVersionSpec::Alias("latest".into()),
                res: None,
            }
        );
    }

    #[test]
    fn parses_latest_with_backend() {
        assert_eq!(
            ToolSpec::parse("asdf:latest").unwrap(),
            ToolSpec {
                backend: Some(Backend::Asdf),
                req: UnresolvedVersionSpec::Alias("latest".into()),
                res: None,
            }
        );
        assert_eq!(
            ToolSpec::parse("proto:latest").unwrap(),
            ToolSpec {
                backend: None,
                req: UnresolvedVersionSpec::Alias("latest".into()),
                res: None,
            }
        );
    }

    #[test]
    fn parses_canary() {
        assert_eq!(
            ToolSpec::parse("canary").unwrap(),
            ToolSpec {
                backend: None,
                req: UnresolvedVersionSpec::parse("canary").unwrap(),
                res: None,
            }
        );
    }

    #[test]
    fn parses_canary_with_backend() {
        assert_eq!(
            ToolSpec::parse("asdf:canary").unwrap(),
            ToolSpec {
                backend: Some(Backend::Asdf),
                req: UnresolvedVersionSpec::Canary,
                res: None,
            }
        );
        assert_eq!(
            ToolSpec::parse("proto:canary").unwrap(),
            ToolSpec {
                backend: None,
                req: UnresolvedVersionSpec::Canary,
                res: None,
            }
        );
    }

    #[test]
    fn parses_calver() {
        assert_eq!(
            ToolSpec::parse("2025-01-01").unwrap(),
            ToolSpec {
                backend: None,
                req: UnresolvedVersionSpec::parse("2025-01-01").unwrap(),
                res: None,
            }
        );
    }

    #[test]
    fn parses_calver_with_backend() {
        assert_eq!(
            ToolSpec::parse("asdf:2025-01-01").unwrap(),
            ToolSpec {
                backend: Some(Backend::Asdf),
                req: UnresolvedVersionSpec::parse("2025-01-01").unwrap(),
                res: None,
            }
        );
        assert_eq!(
            ToolSpec::parse("proto:2025-01-01").unwrap(),
            ToolSpec {
                backend: None,
                req: UnresolvedVersionSpec::parse("2025-01-01").unwrap(),
                res: None,
            }
        );
    }

    #[test]
    fn parses_semver() {
        assert_eq!(
            ToolSpec::parse("1.2.3").unwrap(),
            ToolSpec {
                backend: None,
                req: UnresolvedVersionSpec::parse("1.2.3").unwrap(),
                res: None,
            }
        );
    }

    #[test]
    fn parses_semver_with_backend() {
        assert_eq!(
            ToolSpec::parse("asdf:1.2.3").unwrap(),
            ToolSpec {
                backend: Some(Backend::Asdf),
                req: UnresolvedVersionSpec::parse("1.2.3").unwrap(),
                res: None,
            }
        );
        assert_eq!(
            ToolSpec::parse("proto:1.2.3").unwrap(),
            ToolSpec {
                backend: None,
                req: UnresolvedVersionSpec::parse("1.2.3").unwrap(),
                res: None,
            }
        );
    }

    #[test]
    fn parses_version_req() {
        assert_eq!(
            ToolSpec::parse("^2").unwrap(),
            ToolSpec {
                backend: None,
                req: UnresolvedVersionSpec::parse("^2").unwrap(),
                res: None,
            }
        );
    }

    #[test]
    fn parses_version_req_with_backend() {
        assert_eq!(
            ToolSpec::parse("asdf:^2").unwrap(),
            ToolSpec {
                backend: Some(Backend::Asdf),
                req: UnresolvedVersionSpec::parse("^2").unwrap(),
                res: None,
            }
        );
        assert_eq!(
            ToolSpec::parse("proto:~1.2").unwrap(),
            ToolSpec {
                backend: None,
                req: UnresolvedVersionSpec::parse("~1.2").unwrap(),
                res: None,
            }
        );
    }
}
