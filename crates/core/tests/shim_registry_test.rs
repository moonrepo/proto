// Tests for the `Shim` struct's serde behaviour, with a focus on backwards
// compatibility for the field renames in this branch:
//
//   * `parent: Option<String>`  -> `context: Option<ToolContext>`
//   * `alt_bin: Option<bool>`   -> `alt_exe: Option<bool>`
//
// Both new fields keep the old names as `serde(alias = ...)` so existing
// `~/.proto/shims/registry.json` files continue to parse cleanly. These tests
// pin that contract (and pin the new wire format going forward) without
// touching disk or instantiating a real tool.

use proto_core::layout::Shim;
use proto_core::{Id, ToolContext};

mod shim {
    use super::*;

    // -- Backwards compatibility: old field names ----------------------------
    //
    // These two cases are the most important: they prove that an on-disk
    // registry written by an older proto version is still parsed by the new
    // code via the `serde(alias)` attributes.

    #[test]
    fn deserialises_old_alt_bin_alias() {
        let shim: Shim = serde_json::from_str(r#"{"alt_bin": true}"#).unwrap();
        assert_eq!(shim.alt_exe, Some(true));
    }

    #[test]
    fn deserialises_old_parent_alias_as_tool_context() {
        let shim: Shim = serde_json::from_str(r#"{"parent": "asdf:zig"}"#).unwrap();
        assert_eq!(
            shim.context,
            Some(ToolContext::with_backend(Id::raw("zig"), Id::raw("asdf"))),
        );
    }

    // The original `parent` field was a plain `String` and is most often a
    // bare tool id without a backend prefix. `ToolContext` parses that branch
    // too — make sure it still works.
    #[test]
    fn deserialises_old_parent_alias_without_backend() {
        let shim: Shim = serde_json::from_str(r#"{"parent": "npm"}"#).unwrap();
        assert_eq!(shim.context, Some(ToolContext::new(Id::raw("npm"))));
    }

    // -- Canonical (new) field names ----------------------------------------

    #[test]
    fn deserialises_new_alt_exe_field() {
        let shim: Shim = serde_json::from_str(r#"{"alt_exe": true}"#).unwrap();
        assert_eq!(shim.alt_exe, Some(true));
    }

    #[test]
    fn deserialises_new_context_field() {
        let shim: Shim = serde_json::from_str(r#"{"context": "asdf:zig"}"#).unwrap();
        assert_eq!(
            shim.context,
            Some(ToolContext::with_backend(Id::raw("zig"), Id::raw("asdf"))),
        );
    }

    // -- Wire format: serialising must use the new names --------------------
    //
    // This pins the shape that gets written to `registry.json`. If a future
    // refactor accidentally swaps back to `parent`/`alt_bin`, this test fails
    // and warns us before the rollout.

    #[test]
    fn serialises_with_new_field_names() {
        let shim = Shim {
            alt_exe: Some(true),
            context: Some(ToolContext::new(Id::raw("npm"))),
            ..Default::default()
        };

        let json = serde_json::to_string(&shim).unwrap();

        assert!(
            json.contains(r#""alt_exe":true"#),
            "expected alt_exe in: {json}",
        );
        assert!(
            json.contains(r#""context":"npm""#),
            "expected context in: {json}",
        );
        // And critically: no leftover old keys.
        assert!(!json.contains("alt_bin"), "alt_bin leaked into: {json}");
        assert!(!json.contains("parent"), "parent leaked into: {json}");
    }

    // -- Round-trip ---------------------------------------------------------
    //
    // Parsing an old-format registry, re-serialising, and parsing again must
    // preserve the values — this is what proto does on every shim update.

    #[test]
    fn old_format_round_trips_to_new_format() {
        let old = r#"{"parent": "asdf:zig", "alt_bin": true}"#;

        let parsed: Shim = serde_json::from_str(old).unwrap();
        let reserialised = serde_json::to_string(&parsed).unwrap();
        let reparsed: Shim = serde_json::from_str(&reserialised).unwrap();

        assert_eq!(reparsed.alt_exe, Some(true));
        assert_eq!(
            reparsed.context,
            Some(ToolContext::with_backend(Id::raw("zig"), Id::raw("asdf"))),
        );
        // Re-serialised form must use the new key names.
        assert!(reserialised.contains(r#""alt_exe":true"#));
        assert!(reserialised.contains(r#""context":"asdf:zig""#));
    }
}
