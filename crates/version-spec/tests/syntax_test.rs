use version_spec::{Version, parse_semver};

mod syntax {
    use super::*;

    mod semver {
        use super::*;

        #[test]
        fn parses() {
            assert_eq!(
                parse_semver("0.0.0").unwrap(),
                Version {
                    major: 0,
                    minor: 0,
                    micro: 0,
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver("1.2.3").unwrap(),
                Version {
                    major: 1,
                    minor: 2,
                    micro: 3,
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver("10.20.30").unwrap(),
                Version {
                    major: 10,
                    minor: 20,
                    micro: 30,
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver("123.456.789").unwrap(),
                Version {
                    major: 123,
                    minor: 456,
                    micro: 789,
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_and_trims_whitespace() {
            assert_eq!(
                parse_semver("  1.2.3  ").unwrap(),
                Version {
                    major: 1,
                    minor: 2,
                    micro: 3,
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_max_u64() {
            assert_eq!(
                parse_semver("18446744073709551615.0.0").unwrap().major,
                u64::MAX
            );
        }

        #[test]
        fn parses_pre() {
            for (input, pre) in [
                ("1.2.3-0", "0"),
                ("1.2.3-alpha", "alpha"),
                ("1.2.3-alpha.1", "alpha.1"),
                ("1.2.3-rc.1.2", "rc.1.2"),
                ("1.2.3-beta-2", "beta-2"),
                ("1.2.3-un_stable", "un_stable"),
            ] {
                assert_eq!(
                    parse_semver(input).unwrap(),
                    Version {
                        major: 1,
                        minor: 2,
                        micro: 3,
                        prerelease: Some(pre.into()),
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_build() {
            for (input, build) in [
                ("1.2.3+build", "build"),
                ("1.2.3+build.123", "build.123"),
                ("1.2.3+exp.sha.5114f85", "exp.sha.5114f85"),
            ] {
                assert_eq!(
                    parse_semver(input).unwrap(),
                    Version {
                        major: 1,
                        minor: 2,
                        micro: 3,
                        build: Some(build.into()),
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_pre_and_build() {
            assert_eq!(
                parse_semver("1.2.3-beta.1+exp.sha.5114f85").unwrap(),
                Version {
                    major: 1,
                    minor: 2,
                    micro: 3,
                    prerelease: Some("beta.1".into()),
                    build: Some("exp.sha.5114f85".into()),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_scope() {
            for (input, scope) in [
                ("node-1.2.3", "node"),
                ("foo-bar-1.2.3", "foo-bar"),
                ("foo_bar-1.2.3", "foo_bar"),
                ("a1-1.2.3", "a1"),
                ("v8-1.2.3", "v8"),
                ("node-16-1.2.3", "node-16"),
            ] {
                assert_eq!(
                    parse_semver(input).unwrap(),
                    Version {
                        scope: Some(scope.into()),
                        major: 1,
                        minor: 2,
                        micro: 3,
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }

            // multi-digit version parts
            assert_eq!(
                parse_semver("graalvm-ce-21.0.2").unwrap(),
                Version {
                    scope: Some("graalvm-ce".into()),
                    major: 21,
                    minor: 0,
                    micro: 2,
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_scope_with_trailing_dash() {
            assert_eq!(
                parse_semver("foo--1.2.3").unwrap(),
                Version {
                    scope: Some("foo-".into()),
                    major: 1,
                    minor: 2,
                    micro: 3,
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_scope_with_pre_and_build() {
            assert_eq!(
                parse_semver("node-1.2.3-alpha.1+build.5").unwrap(),
                Version {
                    scope: Some("node".into()),
                    major: 1,
                    minor: 2,
                    micro: 3,
                    prerelease: Some("alpha.1".into()),
                    build: Some("build.5".into()),
                }
            );
        }

        #[test]
        fn prefers_pre_over_scope() {
            // A pre-release is not extracted as a scope
            assert_eq!(
                parse_semver("1.2.3-alpha").unwrap(),
                Version {
                    major: 1,
                    minor: 2,
                    micro: 3,
                    prerelease: Some("alpha".into()),
                    ..Default::default()
                }
            );

            // Even when the pre-release looks like a version
            assert_eq!(
                parse_semver("1.2.3-4.5.6").unwrap(),
                Version {
                    major: 1,
                    minor: 2,
                    micro: 3,
                    prerelease: Some("4.5.6".into()),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn errors_incomplete() {
            assert!(parse_semver("").is_err());
            assert!(parse_semver("1").is_err());
            assert!(parse_semver("1.2").is_err());
            assert!(parse_semver("1.2.").is_err());
        }

        #[test]
        fn errors_too_many_parts() {
            assert!(parse_semver("1.2.3.4").is_err());
        }

        #[test]
        fn errors_leading_zeros() {
            assert!(parse_semver("01.2.3").is_err());
            assert!(parse_semver("1.02.3").is_err());
            assert!(parse_semver("1.2.03").is_err());
        }

        #[test]
        fn errors_invalid_separators() {
            assert!(parse_semver("1-2-3").is_err());
            assert!(parse_semver("1x2x3").is_err());
            assert!(parse_semver("1.2-3").is_err());
        }

        #[test]
        fn errors_aliases() {
            assert!(parse_semver("latest").is_err());
            assert!(parse_semver("node").is_err());
        }

        #[test]
        fn errors_scoped_partials() {
            assert!(parse_semver("node-1").is_err());
            assert!(parse_semver("node-1.2").is_err());
        }

        #[test]
        fn errors_v_prefix() {
            // A leading "v" is removed by `clean_version_string` before parsing
            assert!(parse_semver("v1.2.3").is_err());
        }

        #[test]
        fn errors_dangling_anchors() {
            assert!(parse_semver("1.2.3-").is_err());
            assert!(parse_semver("1.2.3+").is_err());
            assert!(parse_semver("-1.2.3").is_err());
            assert!(parse_semver("+1.2.3").is_err());
        }

        #[test]
        fn errors_trailing_input() {
            assert!(parse_semver("1.2.3abc").is_err());
            assert!(parse_semver("1.2.30 alpha").is_err());
            assert!(parse_semver("1.2.3, 4.5.6").is_err());
            assert!(parse_semver(">=1.2.3").is_err());
        }

        #[test]
        fn errors_number_overflow() {
            // u64::MAX + 1
            let error = parse_semver("18446744073709551616.0.0").unwrap_err();

            assert!(
                error
                    .to_string()
                    .contains("failed to parse major version")
            );
        }
    }
}
