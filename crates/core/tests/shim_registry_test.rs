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

use proto_core::layout::{Shim, ShimRegistry};
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

// Tests for the registry's concurrency guarantees: `save` must merge into the
// current on-disk state instead of overwriting it with a stale in-memory
// snapshot, and writes must be atomic so no reader can ever observe an empty
// or partially written file.
// https://github.com/moonrepo/proto/issues/1057
mod registry {
    use super::*;
    use starbase_sandbox::create_empty_sandbox;
    use std::path::Path;

    fn secondary(tool: &str) -> Shim {
        Shim {
            alt_exe: Some(true),
            context: Some(ToolContext::new(Id::raw(tool))),
            ..Default::default()
        }
    }

    fn registry_keys(path: &Path) -> Vec<String> {
        ShimRegistry::load(path)
            .unwrap()
            .shims
            .into_keys()
            .collect()
    }

    #[test]
    fn save_merges_entries_written_since_load() {
        let sandbox = create_empty_sandbox();
        let path = sandbox.path().join("shims/registry.json");

        // Both registries load the same (missing) file, then race their saves,
        // as concurrent installs of different tools do.
        let mut uv_reg = ShimRegistry::load(&path).unwrap();
        let mut bun_reg = ShimRegistry::load(&path).unwrap();

        uv_reg.update("uv".into(), Shim::default()).unwrap();
        uv_reg.update("uvx".into(), secondary("uv")).unwrap();
        uv_reg.save().unwrap();

        bun_reg.update("bun".into(), Shim::default()).unwrap();
        bun_reg.update("bunx".into(), secondary("bun")).unwrap();
        bun_reg.save().unwrap();

        // The second save must not erase the first save's entries.
        assert_eq!(registry_keys(&path), vec!["bun", "bunx", "uv", "uvx"]);

        // And the in-memory view reflects the merged state after saving.
        assert_eq!(
            bun_reg.shims.into_keys().collect::<Vec<_>>(),
            vec!["bun", "bunx", "uv", "uvx"],
        );
    }

    #[test]
    fn load_treats_empty_file_as_empty_registry() {
        let sandbox = create_empty_sandbox();
        let path = sandbox.path().join("shims/registry.json");

        sandbox.create_file("shims/registry.json", "");

        let registry = ShimRegistry::load(&path).unwrap();

        assert!(registry.shims.is_empty());
    }

    #[test]
    fn save_reapplies_conflict_resolution_against_fresh_state() {
        let sandbox = create_empty_sandbox();
        let path = sandbox.path().join("shims/registry.json");

        // This registry loads before "go" exists on disk, so its own conflict
        // check at `update` time sees nothing to conflict with.
        let mut late_reg = ShimRegistry::load(&path).unwrap();

        // Meanwhile the primary tool claims its own name and saves first.
        let mut go_reg = ShimRegistry::load(&path).unwrap();
        go_reg.update("go".into(), Shim::default()).unwrap();
        go_reg.save().unwrap();

        // The stale registry now tries to take "go" as a secondary executable.
        late_reg.update("go".into(), secondary("xyz")).unwrap();
        late_reg.save().unwrap();

        // The merge must re-run precedence against the fresh file: the primary
        // tool keeps its name.
        let registry = ShimRegistry::load(&path).unwrap();

        assert_eq!(registry.shims["go"], Shim::default());
    }

    #[test]
    fn concurrent_saves_from_many_threads_lose_no_entries() {
        let sandbox = create_empty_sandbox();
        let path = sandbox.path().join("shims/registry.json");

        std::thread::scope(|scope| {
            for i in 0..8 {
                let path = path.clone();

                scope.spawn(move || {
                    let mut registry = ShimRegistry::load(&path).unwrap();

                    registry
                        .update(format!("tool{i}"), Shim::default())
                        .unwrap();
                    registry
                        .update(format!("tool{i}x"), secondary(&format!("tool{i}")))
                        .unwrap();
                    registry.save().unwrap();
                });
            }
        });

        assert_eq!(registry_keys(&path).len(), 16);
    }

    #[test]
    fn unlocked_readers_never_observe_empty_or_partial_content() {
        let sandbox = create_empty_sandbox();
        let path = sandbox.path().join("shims/registry.json");

        // Seed the file so the reader only ever races rewrites, not creation.
        let mut registry = ShimRegistry::load(&path).unwrap();
        registry.update("tool".into(), Shim::default()).unwrap();
        registry.save().unwrap();

        std::thread::scope(|scope| {
            let writer_path = path.clone();

            let writer = scope.spawn(move || {
                // Each iteration changes the entry so every save rewrites.
                for i in 0..200 {
                    let mut registry = ShimRegistry::load(&writer_path).unwrap();
                    let shim = Shim {
                        before_args: vec![format!("--round={i}")],
                        ..Default::default()
                    };

                    registry.update("tool".into(), shim).unwrap();
                    registry.save().unwrap();
                }
            });

            // The shim binary (`main_shim.rs`) reads the registry with a plain
            // unlocked read on every invocation, so it must always see complete
            // JSON. Skip reads that fail outright (a transient state on
            // Windows renames), but any content read must parse.
            while !writer.is_finished() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    assert!(
                        !content.trim().is_empty(),
                        "reader observed an empty registry file"
                    );

                    serde_json::from_str::<serde_json::Value>(&content)
                        .expect("reader observed partially written registry content");
                }
            }

            writer.join().unwrap();
        });
    }
}
