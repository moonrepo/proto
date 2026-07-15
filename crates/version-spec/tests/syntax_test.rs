use version_spec::{Version, parse_calver, parse_semver};

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

            assert!(error.to_string().contains("failed to parse major version"));
        }
    }

    mod calver {
        use super::*;

        #[test]
        fn parses() {
            // Short years are kept as-is, expansion (24 -> 2024)
            // is handled upstream
            for (input, year, month) in [
                ("2024-02", 2024, 2),
                ("2024-2", 2024, 2),
                ("2024-12", 2024, 12),
                ("224-3", 224, 3),
                ("24-03", 24, 3),
                ("4-1", 4, 1),
                ("04-10", 4, 10),
            ] {
                assert_eq!(
                    parse_calver(input).unwrap(),
                    Version {
                        major: year,
                        minor: month,
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_day() {
            for (input, day) in [
                ("2024-02-1", 1),
                ("2024-02-01", 1),
                ("2024-02-09", 9),
                ("2024-02-18", 18),
                ("2024-02-26", 26),
                ("2024-02-30", 30),
                ("2024-02-31", 31),
            ] {
                assert_eq!(
                    parse_calver(input).unwrap(),
                    Version {
                        major: 2024,
                        minor: 2,
                        micro: day,
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_dot_format() {
            // Not supported by the old regex pattern
            assert_eq!(
                parse_calver("2024.02").unwrap(),
                Version {
                    major: 2024,
                    minor: 2,
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_calver("2024.2.26").unwrap(),
                Version {
                    major: 2024,
                    minor: 2,
                    micro: 26,
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_calver("24.12").unwrap(),
                Version {
                    major: 24,
                    minor: 12,
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_and_trims_whitespace() {
            assert_eq!(
                parse_calver("  2024-02  ").unwrap(),
                Version {
                    major: 2024,
                    minor: 2,
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_build() {
            assert_eq!(
                parse_calver("2024-02+build").unwrap(),
                Version {
                    major: 2024,
                    minor: 2,
                    build: Some("build".into()),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_pre() {
            for (input, day, pre) in [
                ("2024-02-rc.1", 0, "rc.1"),
                ("2024-2-alpha", 0, "alpha"),
                ("2024-02-26-beta.1", 26, "beta.1"),
            ] {
                assert_eq!(
                    parse_calver(input).unwrap(),
                    Version {
                        major: 2024,
                        minor: 2,
                        micro: day,
                        prerelease: Some(pre.into()),
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_scope() {
            assert_eq!(
                parse_calver("node-2024-02").unwrap(),
                Version {
                    scope: Some("node".into()),
                    major: 2024,
                    minor: 2,
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_calver("foo-bar-2024-5-12").unwrap(),
                Version {
                    scope: Some("foo-bar".into()),
                    major: 2024,
                    minor: 5,
                    micro: 12,
                    ..Default::default()
                }
            );

            // short year
            assert_eq!(
                parse_calver("foo_bar-24-1").unwrap(),
                Version {
                    scope: Some("foo_bar".into()),
                    major: 24,
                    minor: 1,
                    ..Default::default()
                }
            );

            // scope segment that looks like a version start
            assert_eq!(
                parse_calver("node-16-2024-02").unwrap(),
                Version {
                    scope: Some("node-16".into()),
                    major: 2024,
                    minor: 2,
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_scope_with_trailing_dash() {
            assert_eq!(
                parse_calver("foo--2024-02").unwrap(),
                Version {
                    scope: Some("foo-".into()),
                    major: 2024,
                    minor: 2,
                    ..Default::default()
                }
            );
        }

        #[test]
        fn prefers_version_over_scope() {
            // The year is not extracted as a scope, even though
            // the string could also match as scope + year-month-pre
            assert_eq!(
                parse_calver("2024-05-1-alpha.1").unwrap(),
                Version {
                    major: 2024,
                    minor: 5,
                    micro: 1,
                    prerelease: Some("alpha.1".into()),
                    ..Default::default()
                }
            );

            // And with a scope, the longest version match wins
            assert_eq!(
                parse_calver("foo-2024-05-1-alpha.1").unwrap(),
                Version {
                    scope: Some("foo".into()),
                    major: 2024,
                    minor: 5,
                    micro: 1,
                    prerelease: Some("alpha.1".into()),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn errors_incomplete() {
            assert!(parse_calver("").is_err());
            assert!(parse_calver("2024").is_err());
            assert!(parse_calver("24").is_err());
            assert!(parse_calver("2024-").is_err());
        }

        #[test]
        fn errors_invalid_months() {
            assert!(parse_calver("2024-0").is_err());
            assert!(parse_calver("2024-00").is_err());
            assert!(parse_calver("2024-13").is_err());
            assert!(parse_calver("2024-20").is_err());
            assert!(parse_calver("2024-010").is_err());
        }

        #[test]
        fn errors_invalid_days() {
            assert!(parse_calver("2024-10-0").is_err());
            assert!(parse_calver("2024-10-00").is_err());
            assert!(parse_calver("2024-10-123").is_err());
            assert!(parse_calver("2024-10-023").is_err());
            assert!(parse_calver("2024-10-40").is_err());
            assert!(parse_calver("2024-10-50").is_err());
        }

        #[test]
        fn errors_invalid_micro() {
            assert!(parse_calver("2024_abc").is_err());
            assert!(parse_calver("2024-10_abc").is_err());
            assert!(parse_calver("2024-1-1_abc").is_err());
        }

        #[test]
        fn errors_scoped_partials() {
            assert!(parse_calver("node-2024").is_err());
            assert!(parse_calver("node-").is_err());
            assert!(parse_calver("foo-bar").is_err());
        }

        #[test]
        fn errors_v_prefix() {
            // A leading "v" is removed by `clean_version_string` before parsing
            assert!(parse_calver("v2024-02").is_err());
        }
    }
}
