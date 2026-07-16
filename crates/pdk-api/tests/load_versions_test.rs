use proto_pdk_api::LoadVersionsOutput;

mod load_versions_output {
    use super::*;

    fn create_output(values: &[&str]) -> LoadVersionsOutput {
        LoadVersionsOutput::from(values.iter().map(|value| value.to_string()).collect()).unwrap()
    }

    #[test]
    fn computes_latest() {
        let output = create_output(&["1.0.0", "2.1.0", "2.0.0"]);
        let latest = output.latest.clone().unwrap();

        assert_eq!(latest.to_string(), "2.1.0");
        assert_eq!(output.aliases.get("latest").unwrap(), &latest);
        assert_eq!(output.versions.len(), 3);
    }

    #[test]
    fn latest_preserves_scope() {
        let output = create_output(&["node-1.2.3", "node-2.0.0", "1.5.0"]);

        assert_eq!(output.latest.unwrap().to_string(), "node-2.0.0");
    }

    #[test]
    fn latest_preserves_calver() {
        let output = create_output(&["2024-02-26", "2023-12-01"]);

        assert_eq!(output.latest.unwrap().to_string(), "2024-02-26");
    }

    #[test]
    fn latest_skips_prereleases() {
        let output = create_output(&["1.0.0", "2.0.0-alpha.1"]);

        assert_eq!(output.latest.unwrap().to_string(), "1.0.0");
    }

    #[test]
    fn latest_falls_back_to_zero() {
        let output = create_output(&[]);

        assert_eq!(output.latest.unwrap().to_string(), "0.0.0");
    }
}
