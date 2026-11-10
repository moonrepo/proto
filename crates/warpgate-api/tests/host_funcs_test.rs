use warpgate_api::{SendRequestInput, SendRequestMethod};

mod send_request_input {
    use super::*;

    #[test]
    fn defaults_to_get_method() {
        let input: SendRequestInput =
            serde_json::from_str(r#"{"url":"https://moonrepo.dev"}"#).unwrap();

        assert_eq!(input.method, SendRequestMethod::Get);
    }

    #[test]
    fn supports_lower_and_upper_case_methods() {
        for (value, method) in [
            ("get", SendRequestMethod::Get),
            ("GET", SendRequestMethod::Get),
            ("post", SendRequestMethod::Post),
            ("POST", SendRequestMethod::Post),
        ] {
            let input: SendRequestInput = serde_json::from_str(&format!(
                r#"{{"url":"https://moonrepo.dev","method":"{value}"}}"#
            ))
            .unwrap();

            assert_eq!(input.method, method);
        }
    }

    #[test]
    fn post_constructor_sets_method() {
        let input = SendRequestInput::post("https://moonrepo.dev");

        assert_eq!(input.method, SendRequestMethod::Post);
    }

    #[test]
    fn errors_on_failure_by_default() {
        let input: SendRequestInput =
            serde_json::from_str(r#"{"url":"https://moonrepo.dev"}"#).unwrap();

        assert!(input.error_on_failure);
        assert!(SendRequestInput::new("https://moonrepo.dev").error_on_failure);
        assert!(SendRequestInput::post("https://moonrepo.dev").error_on_failure);
        assert!(SendRequestInput::default().error_on_failure);
    }

    #[test]
    fn can_disable_error_on_failure() {
        let input: SendRequestInput =
            serde_json::from_str(r#"{"url":"https://moonrepo.dev","error_on_failure":false}"#)
                .unwrap();

        assert!(!input.error_on_failure);
    }
}
