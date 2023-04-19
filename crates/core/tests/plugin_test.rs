use proto_core::PluginLocator;
use std::str::FromStr;

mod config {
    use super::*;

    #[test]
    fn deser_to_enum() {
        let value: PluginLocator =
            serde_json::from_str("\"schema:https://foo.com/file.toml\"").unwrap();

        assert_eq!(
            value,
            PluginLocator::Schema("https://foo.com/file.toml".into())
        );
    }

    #[test]
    fn ser_to_string() {
        assert_eq!(
            serde_json::to_string(&PluginLocator::Schema("https://foo.com/file.toml".into()))
                .unwrap(),
            "\"schema:https://foo.com/file.toml\""
        );
    }

    #[test]
    #[should_panic(expected = "InvalidPluginLocator")]
    fn errors_for_invalid_value() {
        PluginLocator::from_str("invalid").unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidPluginProtocol(\"invalid\")")]
    fn errors_for_invalid_protocol() {
        PluginLocator::from_str("invalid:value").unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidPluginLocator")]
    fn errors_for_empty_location() {
        PluginLocator::from_str("schema:").unwrap();
    }

    mod schema {
        use super::*;

        #[test]
        #[should_panic(expected = "InvalidPluginLocator")]
        fn errors_for_http_url() {
            PluginLocator::from_str("schema:http://foo.com").unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidPluginLocator")]
        fn errors_for_abs_path() {
            PluginLocator::from_str("schema:/foo/file").unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidPluginLocator")]
        fn errors_for_random_value() {
            PluginLocator::from_str("schema:abc123").unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidPluginLocatorExt(\".toml\")")]
        fn errors_for_invalid_url_ext() {
            PluginLocator::from_str("schema:https://foo.com/file.yaml").unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidPluginLocatorExt(\".toml\")")]
        fn errors_for_invalid_path_ext() {
            PluginLocator::from_str("schema:../foo/file.yaml").unwrap();
        }
    }
}
