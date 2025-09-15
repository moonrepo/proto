use proto_core::ToolContext;

mod tool_context {
    use super::*;

    #[test]
    #[should_panic(expected = "ProtoIdError")]
    fn errors_invalid_format() {
        ToolContext::parse("1~a.2").unwrap();
    }

    #[test]
    fn parses_without_backend() {
        assert_eq!(
            ToolContext::parse("tool").unwrap(),
            ToolContext::new("tool".into()),
        );
    }

    #[test]
    fn parses_with_backend() {
        assert_eq!(
            ToolContext::parse("backend:tool").unwrap(),
            ToolContext::with_backend("tool".into(), "backend".into()),
        );
    }

    #[test]
    fn parses_with_missing_backend() {
        assert_eq!(
            ToolContext::parse(":tool").unwrap(),
            ToolContext::new("tool".into()),
        );
        assert_eq!(
            ToolContext::parse("tool:").unwrap(),
            ToolContext::new("tool".into()),
        );
    }

    #[test]
    fn supports_npm_packages() {
        assert_eq!(
            ToolContext::parse("@moonrepo/cli").unwrap(),
            ToolContext::new("@moonrepo/cli".into()),
        );
        assert_eq!(
            ToolContext::parse("npm:@moonrepo/cli").unwrap(),
            ToolContext::with_backend("@moonrepo/cli".into(), "npm".into()),
        );
    }
}
