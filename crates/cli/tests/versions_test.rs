use proto_core::test_utils::*;
use proto_core::{ToolManifest, ToolManifestVersion, VersionSpec};
use starbase_sandbox::output_to_string;
use starbase_sandbox::predicates::prelude::*;

fn list_versions(sandbox: &ProtoSandbox, filter: &str) -> Vec<String> {
    let assert = sandbox.run_bin(|cmd| {
        cmd.arg("versions")
            .arg("protostar")
            .arg(filter)
            .arg("--json");
    });

    let output: serde_json::Value = serde_json::from_str(&assert.stdout()).unwrap();

    output["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["version"].as_str().unwrap().to_owned())
        .collect()
}

mod versions {
    use super::*;

    #[test]
    fn lists_remote_versions() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions").arg("protostar");
        });

        // Without stderr
        let output = output_to_string(&assert.inner.get_output().stdout);
        let length = output.split('\n').collect::<Vec<_>>().len();

        // Run again but filtered
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions").arg("protostar").arg("~1");
        });

        // Without stderr
        let output2 = output_to_string(&assert.inner.get_output().stdout);
        let length2 = output2.split('\n').collect::<Vec<_>>().len();

        assert_ne!(length, length2);
    }

    #[test]
    fn filters_by_requirement() {
        let sandbox = create_empty_proto_sandbox();

        assert_eq!(
            list_versions(&sandbox, "~2.3"),
            (0..=15)
                .map(|patch| format!("2.3.{patch}"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn filters_by_range() {
        let sandbox = create_empty_proto_sandbox();

        assert_eq!(
            list_versions(&sandbox, "~1.2 || ~3.4"),
            (0..=15)
                .map(|patch| format!("1.2.{patch}"))
                .chain((0..=15).map(|patch| format!("3.4.{patch}")))
                .collect::<Vec<_>>()
        );

        assert_eq!(
            list_versions(&sandbox, "2.3.14 - 2.4.1"),
            ["2.3.14", "2.3.15", "2.4.0", "2.4.1"]
        );
    }

    #[test]
    fn filters_by_version() {
        let sandbox = create_empty_proto_sandbox();

        assert_eq!(list_versions(&sandbox, "2.3.4"), ["2.3.4"]);
        assert_eq!(list_versions(&sandbox, "6.0.0-beta.1"), ["6.0.0-beta.1"]);
        assert_eq!(list_versions(&sandbox, "canary"), ["canary"]);
    }

    #[test]
    fn filters_by_alias() {
        let sandbox = create_empty_proto_sandbox();

        // From the list of versions
        assert_eq!(list_versions(&sandbox, "latest"), ["5.10.15"]);

        // From the plugin
        assert_eq!(list_versions(&sandbox, "stable"), ["5.0.0"]);
        assert_eq!(list_versions(&sandbox, "unstable"), ["6.0.0-rc.1"]);

        // From the config, which resolves to the highest matching version
        sandbox.create_file(
            ".prototools",
            r#"
[tools.protostar.aliases]
work = "~2.1"
"#,
        );

        assert_eq!(list_versions(&sandbox, "work"), ["2.1.15"]);
    }

    #[test]
    fn errors_for_unknown_alias() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions").arg("protostar").arg("unknown");
        });

        assert.failure().stderr(predicate::str::contains(
            "Failed to resolve unknown to a valid supported version",
        ));
    }

    #[test]
    fn lists_local_versions() {
        let sandbox = create_empty_proto_sandbox();
        let versions = vec!["1.0.0", "2.0.0", "3.0.0"];

        let mut manifest =
            ToolManifest::load(sandbox.path().join(".proto/tools/protostar/manifest.json"))
                .unwrap();

        for version in &versions {
            manifest.versions.insert(
                VersionSpec::parse(version).unwrap(),
                ToolManifestVersion::default(),
            );
        }

        manifest.save().unwrap();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions").arg("protostar");
        });

        // Without stderr
        let output = output_to_string(&assert.inner.get_output().stdout);
        let mut count = 0;

        for line in output.lines() {
            for version in &versions {
                if line.starts_with(version) {
                    count += 1;
                    assert!(line.contains("installed"));
                }
            }
        }

        assert_eq!(count, 3);
    }

    #[test]
    fn only_displays_local_versions() {
        let sandbox = create_empty_proto_sandbox();
        let versions = vec!["1.0.0", "2.0.0", "3.0.0"];

        let mut manifest =
            ToolManifest::load(sandbox.path().join(".proto/tools/protostar/manifest.json"))
                .unwrap();

        for version in &versions {
            manifest.versions.insert(
                VersionSpec::parse(version).unwrap(),
                ToolManifestVersion::default(),
            );
        }

        manifest.save().unwrap();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions").arg("protostar").arg("--installed");
        });

        // Without stderr
        let output = output_to_string(&assert.inner.get_output().stdout);

        assert_eq!(output.lines().collect::<Vec<_>>().len(), 3);
    }

    // Windows doesn't support asdf
    #[cfg(unix)]
    mod backend {
        use super::*;

        #[test]
        fn lists_remote_versions() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("versions").arg("asdf:zig");
            });

            // Without stderr
            let output = output_to_string(&assert.inner.get_output().stdout);

            assert!(output.split('\n').collect::<Vec<_>>().len() > 1);
        }
    }
}
