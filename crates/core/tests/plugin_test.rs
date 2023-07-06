use proto_core::{PluginLocation, PluginLocator};
use std::str::FromStr;

mod config {
    use super::*;

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
        PluginLocator::from_str("source:").unwrap();
    }

    mod schema {
        use super::*;

        #[test]
        fn deser_to_enum() {
            let value: PluginLocator =
                serde_json::from_str("\"source:https://foo.com/file.toml\"").unwrap();

            assert_eq!(
                value,
                PluginLocator::Source(PluginLocation::Url("https://foo.com/file.toml".into()))
            );
        }

        #[test]
        fn ser_to_string() {
            assert_eq!(
                serde_json::to_string(&PluginLocator::Source(PluginLocation::Url(
                    "https://foo.com/file.toml".into()
                )))
                .unwrap(),
                "\"source:https://foo.com/file.toml\""
            );
        }

        #[test]
        #[should_panic(expected = "InvalidPluginLocator")]
        fn errors_for_http_url() {
            PluginLocator::from_str("source:http://foo.com").unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidPluginLocator")]
        fn errors_for_abs_path() {
            PluginLocator::from_str("source:/foo/file").unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidPluginLocator")]
        fn errors_for_random_value() {
            PluginLocator::from_str("source:abc123").unwrap();
        }
    }

    mod source {
        use super::*;

        #[test]
        fn deser_to_enum() {
            let value: PluginLocator =
                serde_json::from_str("\"source:https://foo.com/file.wasm\"").unwrap();

            assert_eq!(
                value,
                PluginLocator::Source(PluginLocation::Url("https://foo.com/file.wasm".into()))
            );
        }

        #[test]
        fn ser_to_string() {
            assert_eq!(
                serde_json::to_string(&PluginLocator::Source(PluginLocation::Url(
                    "https://foo.com/file.wasm".into()
                )))
                .unwrap(),
                "\"source:https://foo.com/file.wasm\""
            );
        }

        #[test]
        #[should_panic(expected = "InvalidPluginLocator")]
        fn errors_for_http_url() {
            PluginLocator::from_str("source:http://foo.com").unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidPluginLocator")]
        fn errors_for_abs_path() {
            PluginLocator::from_str("source:/foo/file").unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidPluginLocator")]
        fn errors_for_random_value() {
            PluginLocator::from_str("source:abc123").unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidPluginLocatorExt(\".wasm OR .toml\")")]
        fn errors_for_invalid_url_ext() {
            PluginLocator::from_str("source:https://foo.com/file.yaml").unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidPluginLocatorExt(\".wasm OR .toml\")")]
        fn errors_for_invalid_path_ext() {
            PluginLocator::from_str("source:../foo/file.yaml").unwrap();
        }
    }
}
