use serde_json::{Value, json};
use std::borrow::Cow;
use warpgate_api::select_json_path;

fn create_index() -> Value {
    json!({
        "name": "tool",
        "latest": {
            "beta": "3.0.0-beta",
            "stable": "2.0.0"
        },
        "releases": [
            { "version": "1.0.0", "assets": ["a", "b"], "notes": "first" },
            { "version": "2.0.0", "assets": ["c"] }
        ],
        "empty": [],
        "nothing": {}
    })
}

fn select_strings(root: &Value, path: &str) -> Vec<String> {
    select_json_path(root, path)
        .unwrap()
        .into_iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect()
}

mod select_json_path {
    use super::*;

    #[test]
    fn returns_root_for_empty_paths() {
        let index = create_index();

        assert_eq!(
            select_json_path(&index, "").unwrap(),
            [Cow::Borrowed(&index)]
        );
        assert_eq!(
            select_json_path(&index, "$").unwrap(),
            [Cow::Borrowed(&index)]
        );
        assert_eq!(
            select_json_path(&index, ".").unwrap(),
            [Cow::Borrowed(&index)]
        );
    }

    #[test]
    fn selects_keys() {
        let index = create_index();

        assert_eq!(select_strings(&index, "name"), ["tool"]);
        assert_eq!(select_strings(&index, ".name"), ["tool"]);
        assert_eq!(select_strings(&index, "$.name"), ["tool"]);
    }

    #[test]
    fn selects_nested_keys() {
        let index = create_index();

        assert_eq!(select_strings(&index, "latest.stable"), ["2.0.0"]);
        assert_eq!(select_strings(&index, "$.latest.beta"), ["3.0.0-beta"]);
    }

    #[test]
    fn selects_array_indexes() {
        let index = create_index();

        assert_eq!(select_strings(&index, "releases[0].version"), ["1.0.0"]);
        assert_eq!(select_strings(&index, "$.releases[1].version"), ["2.0.0"]);
        assert_eq!(select_strings(&index, "releases[0].assets[1]"), ["b"]);
    }

    #[test]
    fn selects_indexes_from_root_array() {
        let index = json!(["a", "b", "c"]);

        assert_eq!(select_strings(&index, "[2]"), ["c"]);
        assert_eq!(select_strings(&index, ".[1]"), ["b"]);
        assert_eq!(select_strings(&index, "$[0]"), ["a"]);
    }

    #[test]
    fn ignores_empty_segments() {
        let index = create_index();

        assert_eq!(select_strings(&index, "..name"), ["tool"]);
        assert_eq!(select_strings(&index, "releases.[0].version"), ["1.0.0"]);
        assert_eq!(select_strings(&index, "latest.stable."), ["2.0.0"]);
    }

    #[test]
    fn returns_arrays_as_a_single_value() {
        let index = create_index();

        assert_eq!(
            select_json_path(&index, "releases[0].assets").unwrap(),
            [Cow::Borrowed(&json!(["a", "b"]))]
        );
    }

    #[test]
    fn selects_array_items_with_wildcards() {
        let index = create_index();

        for path in [
            "releases[*].version",
            "releases.*.version",
            "$.releases[*].version",
        ] {
            assert_eq!(select_strings(&index, path), ["1.0.0", "2.0.0"], "{path}");
        }
    }

    #[test]
    fn selects_object_values_with_wildcards() {
        let index = create_index();

        assert_eq!(select_strings(&index, "latest.*"), ["3.0.0-beta", "2.0.0"]);
        assert_eq!(select_strings(&index, "latest[*]"), ["3.0.0-beta", "2.0.0"]);
    }

    #[test]
    fn selects_from_root_array_with_wildcards() {
        let index = json!([
            { "name": "v2.4.0", "version": "v1.2.3" },
            { "name": "v2.4.1", "version": "v1.2.4" }
        ]);

        assert_eq!(select_strings(&index, "*.version"), ["v1.2.3", "v1.2.4"]);
        assert_eq!(select_strings(&index, "[*].name"), ["v2.4.0", "v2.4.1"]);
        assert_eq!(select_strings(&index, "$.*.name"), ["v2.4.0", "v2.4.1"]);
    }

    #[test]
    fn flattens_nested_wildcards() {
        let index = create_index();

        assert_eq!(
            select_strings(&index, "releases[*].assets[*]"),
            ["a", "b", "c"]
        );
        assert_eq!(
            select_strings(&index, "releases.*.assets.*"),
            ["a", "b", "c"]
        );
    }

    #[test]
    fn selects_object_keys_with_wildcards() {
        let index = create_index();

        assert_eq!(select_strings(&index, "latest.*~"), ["beta", "stable"]);
        assert_eq!(select_strings(&index, "latest[*]~"), ["beta", "stable"]);
        assert_eq!(select_strings(&index, "$.latest.*~"), ["beta", "stable"]);
    }

    #[test]
    fn selects_keys_from_root_object() {
        let index = json!({ "1.0.0": {}, "1.1.0": { "date": "2026-01-01" } });

        assert_eq!(select_strings(&index, "*~"), ["1.0.0", "1.1.0"]);
        assert_eq!(select_strings(&index, "[*]~"), ["1.0.0", "1.1.0"]);
        assert_eq!(select_strings(&index, "$.*~"), ["1.0.0", "1.1.0"]);
    }

    #[test]
    fn selects_keys_of_every_wildcard_value() {
        let index = json!([{ "a": 1 }, { "b": 2, "c": 3 }]);

        assert_eq!(select_strings(&index, "[*].*~"), ["a", "b", "c"]);
    }

    #[test]
    fn errors_for_invalid_paths() {
        let index = create_index();

        for path in [
            "releases[0",
            "releases[",
            "releases]",
            "]",
            "releases[]",
            "releases[abc]",
            "releases[-1]",
            "releases[ 0 ]",
            "releases[0]]",
            // Keys that aren't selected last
            "*~.version",
            "latest.*~[0]",
            "[*]~.name",
        ] {
            assert_eq!(
                select_json_path(&index, path).unwrap_err().to_string(),
                format!("Invalid JSON path {path:?}"),
                "{path}"
            );
        }
    }

    #[test]
    fn errors_when_path_does_not_match() {
        let index = create_index();

        for path in [
            // Missing keys
            "missing",
            "latest.missing",
            // Out of bounds
            "releases[2]",
            "releases[0].assets[5]",
            // Keys on non-objects
            "name.length",
            "releases.version",
            // Indexes on non-arrays
            "name[0]",
            "latest[0]",
            // Wildcards on scalars
            "name.*",
            "name[*]",
            // Wildcards on empty collections
            "empty[*]",
            "nothing.*",
            // Keys missing from some wildcard values
            "releases[*].notes",
            // Keys of non-objects
            "name.*~",
            "releases[*]~",
            // Keys of empty objects
            "nothing.*~",
        ] {
            assert_eq!(
                select_json_path(&index, path).unwrap_err().to_string(),
                format!("JSON path {path:?} did not match"),
                "{path}"
            );
        }
    }
}
