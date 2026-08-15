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
}
