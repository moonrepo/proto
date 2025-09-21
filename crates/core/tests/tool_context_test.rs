use proto_core::{Id, ToolContext};

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
            ToolContext::new(Id::raw("tool")),
        );
    }

    #[test]
    fn parses_with_backend() {
        assert_eq!(
            ToolContext::parse("backend:tool").unwrap(),
            ToolContext::with_backend(Id::raw("tool"), Id::raw("backend")),
        );
    }

    #[test]
    fn parses_with_missing_backend() {
        assert_eq!(
            ToolContext::parse(":tool").unwrap(),
            ToolContext::new(Id::raw("tool")),
        );
        assert_eq!(
            ToolContext::parse("tool:").unwrap(),
            ToolContext::new(Id::raw("tool")),
        );
    }

    #[test]
    fn supports_npm_packages() {
        assert_eq!(
            ToolContext::parse("@moonrepo/cli").unwrap(),
            ToolContext::new(Id::raw("@moonrepo/cli")),
        );
        assert_eq!(
            ToolContext::parse("npm:@moonrepo/cli").unwrap(),
            ToolContext::with_backend(Id::raw("@moonrepo/cli"), Id::raw("npm")),
        );
    }
}
