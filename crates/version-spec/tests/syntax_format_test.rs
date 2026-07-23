use version_spec::{FormatOptions, FormatsVersion, Requirement, Version};

fn ver(input: &str) -> Version {
    Version::parse(input).unwrap()
}

fn req(input: &str) -> Requirement {
    Requirement::parse(input).unwrap()
}

mod syntax_format {
    use super::*;

    mod version {
        use super::*;

        #[test]
        fn includes_all_parts_by_default() {
            assert_eq!(
                ver("node-1.2.3-alpha.1+build.5").to_formatted_string(&FormatOptions::default()),
                "node-1.2.3-alpha.1+build.5"
            );
        }

        #[test]
        fn excludes_minor() {
            // Also excludes the patch, as it requires the minor
            assert_eq!(
                ver("1.2.3").to_formatted_string(&FormatOptions {
                    include_minor: false,
                    ..Default::default()
                }),
                "1"
            );

            // A major-only format, as used by Java-like tools
            assert_eq!(
                ver("temurin-21.0.2-beta").to_formatted_string(&FormatOptions {
                    include_minor: false,
                    include_prerelease: false,
                    ..Default::default()
                }),
                "temurin-21"
            );
        }

        #[test]
        fn excludes_patch() {
            assert_eq!(
                ver("1.2.3").to_formatted_string(&FormatOptions {
                    include_patch: false,
                    ..Default::default()
                }),
                "1.2"
            );
        }

        #[test]
        fn excludes_scope() {
            assert_eq!(
                ver("node-1.2.3").to_formatted_string(&FormatOptions {
                    include_scope: false,
                    ..Default::default()
                }),
                "1.2.3"
            );
        }

        #[test]
        fn excludes_prerelease_and_build() {
            let version = ver("1.2.3-alpha.1+build.5");

            assert_eq!(
                version.to_formatted_string(&FormatOptions {
                    include_prerelease: false,
                    ..Default::default()
                }),
                "1.2.3+build.5"
            );
            assert_eq!(
                version.to_formatted_string(&FormatOptions {
                    include_build: false,
                    ..Default::default()
                }),
                "1.2.3-alpha.1"
            );
            assert_eq!(
                version.to_formatted_string(&FormatOptions {
                    include_prerelease: false,
                    include_build: false,
                    ..Default::default()
                }),
                "1.2.3"
            );
        }

        #[test]
        fn pads_parts() {
            assert_eq!(
                ver("1.2.3").to_formatted_string(&FormatOptions {
                    pad_major: Some(4),
                    pad_minor: Some(2),
                    pad_patch: Some(2),
                    ..Default::default()
                }),
                "0001.02.03"
            );

            // Values wider than the pad width are not truncated
            assert_eq!(
                ver("2024.12.26").to_formatted_string(&FormatOptions {
                    pad_major: Some(2),
                    pad_minor: Some(2),
                    pad_patch: Some(2),
                    ..Default::default()
                }),
                "2024.12.26"
            );
        }

        #[test]
        fn custom_separator() {
            assert_eq!(
                ver("1.2.3").to_formatted_string(&FormatOptions {
                    separator: '_',
                    ..Default::default()
                }),
                "1_2_3"
            );
        }

        #[test]
        fn calendar_options() {
            assert_eq!(
                ver("2024-2-3").to_formatted_string(&FormatOptions::calendar()),
                "2024-02-03"
            );
            assert_eq!(
                ver("node-2024-02").to_formatted_string(&FormatOptions::calendar()),
                "node-2024-02"
            );
        }
    }

    mod requirement {
        use super::*;

        #[test]
        fn includes_all_parts_by_default() {
            assert_eq!(
                req("^temurin-1.2.3-beta.1").to_formatted_string(&FormatOptions::default()),
                "^temurin-1.2.3-beta.1"
            );

            // An omitted operator is normalized to a tilde
            assert_eq!(
                req("1.2").to_formatted_string(&FormatOptions::default()),
                "~1.2"
            );
        }

        #[test]
        fn includes_wildcards_by_default() {
            for (input, expected) in [("*", "*"), ("1.*", "1.*"), ("1.2.*", "1.2.*")] {
                assert_eq!(
                    req(input).to_formatted_string(&FormatOptions::default()),
                    expected,
                    "input: {input}"
                );
            }
        }

        #[test]
        fn excludes_op() {
            for (input, expected) in [
                ("=1", "1"),
                ("~1.2", "1.2"),
                (">=1.2.3", "1.2.3"),
                ("^1.2.3-beta.1", "1.2.3-beta.1"),
                // Wildcard placeholders are also excluded
                ("*", ""),
                ("1.*", "1"),
                ("1.2.*", "1.2"),
            ] {
                assert_eq!(
                    req(input).to_formatted_string(&FormatOptions {
                        include_op: false,
                        ..Default::default()
                    }),
                    expected,
                    "input: {input}"
                );
            }
        }

        #[test]
        fn excludes_minor() {
            // Also excludes the patch, as it requires the minor
            assert_eq!(
                req("^1.2.3").to_formatted_string(&FormatOptions {
                    include_minor: false,
                    ..Default::default()
                }),
                "^1"
            );

            // No dangling wildcard placeholder
            assert_eq!(
                req("1.*").to_formatted_string(&FormatOptions {
                    include_minor: false,
                    ..Default::default()
                }),
                "1"
            );
        }

        #[test]
        fn excludes_patch() {
            assert_eq!(
                req("^1.2.3").to_formatted_string(&FormatOptions {
                    include_patch: false,
                    ..Default::default()
                }),
                "^1.2"
            );

            // No dangling wildcard placeholder
            assert_eq!(
                req("1.2.*").to_formatted_string(&FormatOptions {
                    include_patch: false,
                    ..Default::default()
                }),
                "1.2"
            );
        }

        #[test]
        fn excludes_scope() {
            assert_eq!(
                req("=temurin-21.0.2").to_formatted_string(&FormatOptions {
                    include_scope: false,
                    ..Default::default()
                }),
                "=21.0.2"
            );
        }

        #[test]
        fn excludes_prerelease() {
            assert_eq!(
                req("^1.2.3-beta.1").to_formatted_string(&FormatOptions {
                    include_prerelease: false,
                    ..Default::default()
                }),
                "^1.2.3"
            );
        }

        #[test]
        fn calendar_options() {
            assert_eq!(
                req(">=2000-2-3").to_formatted_string(&FormatOptions::calendar()),
                ">=2000-02-03"
            );
            assert_eq!(
                req("=2000-02").to_formatted_string(&FormatOptions::calendar()),
                "=2000-02"
            );
        }
    }
}
