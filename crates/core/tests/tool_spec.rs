use proto_core::{ToolSpec, UnresolvedVersionSpec};

mod tool_spec {
    use super::*;

    #[test]
    #[should_panic(expected = "InvalidVersionSpec")]
    fn errors_invalid_spec() {
        ToolSpec::parse("1.a.2").unwrap();
    }

    #[test]
    fn parses_latest() {
        assert_eq!(
            ToolSpec::parse("latest").unwrap(),
            ToolSpec {
                req: UnresolvedVersionSpec::Alias("latest".into()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn parses_canary() {
        assert_eq!(
            ToolSpec::parse("canary").unwrap(),
            ToolSpec {
                req: UnresolvedVersionSpec::parse("canary").unwrap(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn parses_calver() {
        assert_eq!(
            ToolSpec::parse("2025-01-01").unwrap(),
            ToolSpec {
                req: UnresolvedVersionSpec::parse("2025-01-01").unwrap(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn parses_semver() {
        assert_eq!(
            ToolSpec::parse("1.2.3").unwrap(),
            ToolSpec {
                req: UnresolvedVersionSpec::parse("1.2.3").unwrap(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn parses_version_req() {
        assert_eq!(
            ToolSpec::parse("^2").unwrap(),
            ToolSpec {
                req: UnresolvedVersionSpec::parse("^2").unwrap(),
                ..Default::default()
            }
        );
    }
}
