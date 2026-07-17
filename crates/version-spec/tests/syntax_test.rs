use version_spec::{
    Clause, MatchesVersion, Op, Range, Requirement, Version, VersionKind, parse_calver,
    parse_calver_range, parse_calver_req, parse_semver, parse_semver_range, parse_semver_req,
};

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
                    patch: 0,
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver("1.2.3").unwrap(),
                Version {
                    major: 1,
                    minor: 2,
                    patch: 3,
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver("10.20.30").unwrap(),
                Version {
                    major: 10,
                    minor: 20,
                    patch: 30,
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver("123.456.789").unwrap(),
                Version {
                    major: 123,
                    minor: 456,
                    patch: 789,
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
                    patch: 3,
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_max_u32() {
            assert_eq!(parse_semver("4294967295.0.0").unwrap().major, u32::MAX);
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
                        patch: 3,
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
                        patch: 3,
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
                    patch: 3,
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
                        patch: 3,
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
                    patch: 2,
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
                    patch: 3,
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
                    patch: 3,
                    prerelease: Some("alpha.1".into()),
                    build: Some("build.5".into()),
                    ..Default::default()
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
                    patch: 3,
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
                    patch: 3,
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
        fn parses_v_prefix() {
            // A leading "v" or "V" is ignored
            for input in ["v1.2.3", "V1.2.3"] {
                assert_eq!(
                    parse_semver(input).unwrap(),
                    Version {
                        major: 1,
                        minor: 2,
                        patch: 3,
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
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
            // u32::MAX + 1
            let error = parse_semver("4294967296.0.0").unwrap_err();

            assert!(error.to_string().contains("failed to parse major version"));
        }
    }

    mod semver_req {
        use super::*;

        #[test]
        fn parses_wildcard() {
            for input in ["", "*", "  *  ", "x", "X"] {
                assert_eq!(
                    parse_semver_req(input).unwrap(),
                    Requirement {
                        op: Op::Wildcard,
                        ..Default::default()
                    },
                    "input: {input:?}"
                );
            }
        }

        #[test]
        fn parses_partial() {
            // No operator defaults to a tilde match
            assert_eq!(
                parse_semver_req("1").unwrap(),
                Requirement {
                    major: Some(1),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver_req("0").unwrap(),
                Requirement {
                    major: Some(0),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver_req("1.2").unwrap(),
                Requirement {
                    major: Some(1),
                    minor: Some(2),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_full() {
            assert_eq!(
                parse_semver_req("1.2.3").unwrap(),
                Requirement {
                    major: Some(1),
                    minor: Some(2),
                    patch: Some(3),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver_req("10.20.30").unwrap(),
                Requirement {
                    major: Some(10),
                    minor: Some(20),
                    patch: Some(30),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_part_wildcards() {
            // A wildcard part matches any value, and marks the
            // whole requirement as a wildcard match
            for (input, major, minor) in [
                ("1.*", Some(1), None),
                ("1.x", Some(1), None),
                ("1.X", Some(1), None),
                ("1.2.*", Some(1), Some(2)),
                ("1.2.x", Some(1), Some(2)),
                ("1.2.X", Some(1), Some(2)),
            ] {
                assert_eq!(
                    parse_semver_req(input).unwrap(),
                    Requirement {
                        op: Op::Wildcard,
                        major,
                        minor,
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_ops() {
            for (input, op) in [
                ("=1.2.3", Op::Exact),
                ("==1.2.3", Op::Exact),
                (">1.2.3", Op::Greater),
                (">=1.2.3", Op::GreaterEq),
                ("<1.2.3", Op::Less),
                ("<=1.2.3", Op::LessEq),
                ("~1.2.3", Op::Tilde),
                ("^1.2.3", Op::Caret),
            ] {
                assert_eq!(
                    parse_semver_req(input).unwrap(),
                    Requirement {
                        op,
                        major: Some(1),
                        minor: Some(2),
                        patch: Some(3),
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_ops_with_partial() {
            assert_eq!(
                parse_semver_req(">=1").unwrap(),
                Requirement {
                    op: Op::GreaterEq,
                    major: Some(1),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver_req("~1.2").unwrap(),
                Requirement {
                    op: Op::Tilde,
                    major: Some(1),
                    minor: Some(2),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver_req("^0.5").unwrap(),
                Requirement {
                    op: Op::Caret,
                    major: Some(0),
                    minor: Some(5),
                    ..Default::default()
                }
            );

            // with a wildcard part
            assert_eq!(
                parse_semver_req(">=1.x").unwrap(),
                Requirement {
                    op: Op::GreaterEq,
                    major: Some(1),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_op_with_whitespace() {
            for (input, op) in [
                ("= 1.2.3", Op::Exact),
                ("> 1.2.3", Op::Greater),
                (">= 1.2.3", Op::GreaterEq),
                ("<  1.2.3", Op::Less),
                ("<=  1.2.3", Op::LessEq),
                ("~ 1.2.3", Op::Tilde),
                ("^ 1.2.3", Op::Caret),
            ] {
                assert_eq!(
                    parse_semver_req(input).unwrap(),
                    Requirement {
                        op,
                        major: Some(1),
                        minor: Some(2),
                        patch: Some(3),
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_pre() {
            assert_eq!(
                parse_semver_req("1.2.3-alpha").unwrap(),
                Requirement {
                    major: Some(1),
                    minor: Some(2),
                    patch: Some(3),
                    prerelease: Some("alpha".into()),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver_req("<2.4.0-0").unwrap(),
                Requirement {
                    op: Op::Less,
                    major: Some(2),
                    minor: Some(4),
                    patch: Some(0),
                    prerelease: Some("0".into()),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver_req("^1.2.3-beta.1").unwrap(),
                Requirement {
                    op: Op::Caret,
                    major: Some(1),
                    minor: Some(2),
                    patch: Some(3),
                    prerelease: Some("beta.1".into()),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_and_trims_whitespace() {
            assert_eq!(
                parse_semver_req("  >=1.2  ").unwrap(),
                Requirement {
                    op: Op::GreaterEq,
                    major: Some(1),
                    minor: Some(2),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn errors_missing_version() {
            assert!(parse_semver_req(">=").is_err());
            assert!(parse_semver_req("~").is_err());
            assert!(parse_semver_req("^").is_err());
            assert!(parse_semver_req("1.").is_err());
            assert!(parse_semver_req("node-").is_err());
        }

        #[test]
        fn errors_invalid_ops() {
            assert!(parse_semver_req("=>1.2").is_err());
            assert!(parse_semver_req("=<1.2").is_err());
            assert!(parse_semver_req(">>1.2").is_err());
            assert!(parse_semver_req("!=1.2").is_err());
        }

        #[test]
        fn errors_interior_whitespace() {
            // Whitespace is only allowed after the operator
            assert!(parse_semver_req("1.2 .3").is_err());
            assert!(parse_semver_req("1 .2.3").is_err());
            assert!(parse_semver_req("> =1.2").is_err());
        }

        #[test]
        fn errors_multiple_reqs() {
            // Comma/space separated requirement lists are split upstream
            assert!(parse_semver_req("1.2, 3.4").is_err());
            assert!(parse_semver_req(">=1.2.7 <1.3.0").is_err());
            assert!(parse_semver_req("1.2 || 3.4").is_err());
        }

        #[test]
        fn parses_scope() {
            for (input, scope, major, minor, patch) in [
                ("node-1.2.3", "node", Some(1), Some(2), Some(3)),
                ("node-1", "node", Some(1), None, None),
                ("gcc-12", "gcc", Some(12), None, None),
                ("foo-bar-1.2", "foo-bar", Some(1), Some(2), None),
                ("foo_bar-1.2", "foo_bar", Some(1), Some(2), None),
                ("v8-10.1", "v8", Some(10), Some(1), None),
                ("node-16-1.2", "node-16", Some(1), Some(2), None),
            ] {
                assert_eq!(
                    parse_semver_req(input).unwrap(),
                    Requirement {
                        scope: Some(scope.into()),
                        major,
                        minor,
                        patch,
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_scope_with_part_wildcard() {
            assert_eq!(
                parse_semver_req("node-1.x").unwrap(),
                Requirement {
                    op: Op::Wildcard,
                    scope: Some("node".into()),
                    major: Some(1),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_scope_with_trailing_dash() {
            assert_eq!(
                parse_semver_req("foo--1.2").unwrap(),
                Requirement {
                    scope: Some("foo-".into()),
                    major: Some(1),
                    minor: Some(2),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_scope_with_op() {
            assert_eq!(
                parse_semver_req(">=node-1.2").unwrap(),
                Requirement {
                    op: Op::GreaterEq,
                    scope: Some("node".into()),
                    major: Some(1),
                    minor: Some(2),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver_req("~ node-16").unwrap(),
                Requirement {
                    op: Op::Tilde,
                    scope: Some("node".into()),
                    major: Some(16),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_scope_with_pre() {
            assert_eq!(
                parse_semver_req("node-1.2.3-alpha").unwrap(),
                Requirement {
                    scope: Some("node".into()),
                    major: Some(1),
                    minor: Some(2),
                    patch: Some(3),
                    prerelease: Some("alpha".into()),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_semver_req("node-1.2-rc.1").unwrap(),
                Requirement {
                    scope: Some("node".into()),
                    major: Some(1),
                    minor: Some(2),
                    prerelease: Some("rc.1".into()),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_scope_with_wildcard() {
            for input in ["node-*", "node-x", "node-X"] {
                assert_eq!(
                    parse_semver_req(input).unwrap(),
                    Requirement {
                        op: Op::Wildcard,
                        scope: Some("node".into()),
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_v_prefix() {
            // A leading "v" or "V" is ignored
            assert_eq!(
                parse_semver_req("v1.2.3").unwrap(),
                parse_semver_req("1.2.3").unwrap()
            );
            assert_eq!(
                parse_semver_req(">=V1.2").unwrap(),
                parse_semver_req(">=1.2").unwrap()
            );
            assert_eq!(
                parse_semver_req("~ v1").unwrap(),
                parse_semver_req("~1").unwrap()
            );
        }

        #[test]
        fn errors_leading_zeros() {
            assert!(parse_semver_req("01").is_err());
            assert!(parse_semver_req("1.02").is_err());
            assert!(parse_semver_req("1.2.03").is_err());
        }

        #[test]
        fn errors_too_many_parts() {
            assert!(parse_semver_req("1.2.3.4").is_err());
        }

        #[test]
        fn errors_dangling_anchors() {
            assert!(parse_semver_req("1.2.3-").is_err());
            assert!(parse_semver_req("1.2.3+").is_err());
        }

        #[test]
        fn parses_and_ignores_build_metadata() {
            assert_eq!(
                parse_semver_req("1.2.3+build").unwrap(),
                parse_semver_req("1.2.3").unwrap()
            );
            assert_eq!(
                parse_semver_req(">=1.2.3-alpha+build").unwrap(),
                parse_semver_req(">=1.2.3-alpha").unwrap()
            );
            assert_eq!(
                parse_semver_req("node-1.2+build.5").unwrap(),
                parse_semver_req("node-1.2").unwrap()
            );
        }

        #[test]
        fn errors_number_overflow() {
            // u32::MAX + 1
            let error = parse_semver_req("4294967296").unwrap_err();

            assert!(error.to_string().contains("failed to parse major version"));
        }
    }

    mod semver_range {
        use super::*;

        fn req(input: &str) -> Requirement {
            parse_semver_req(input).unwrap()
        }

        fn ver(input: &str) -> Box<Version> {
            Box::new(parse_semver(input).unwrap())
        }

        #[test]
        fn parses_single() {
            assert_eq!(
                parse_semver_range("^1.2").unwrap(),
                Range {
                    clauses: vec![Clause::Only(Requirement {
                        op: Op::Caret,
                        major: Some(1),
                        minor: Some(2),
                        ..Default::default()
                    })]
                }
            );

            assert_eq!(
                parse_semver_range("1.2.3").unwrap(),
                Range {
                    clauses: vec![Clause::Only(req("1.2.3"))]
                }
            );
        }

        #[test]
        fn parses_wildcard() {
            for input in ["", "*", "  *  ", "x", "X"] {
                assert_eq!(
                    parse_semver_range(input).unwrap(),
                    Range::default(),
                    "input: {input:?}"
                );
            }
        }

        #[test]
        fn parses_and() {
            // " ", "&&", and "," are supported
            for input in [
                "^1 && <1.5",
                "^1, <1.5",
                "^1,<1.5",
                "^1&&<1.5",
                "^1  &&  <1.5",
                "^1 <1.5",
                "^1  <1.5",
                "^1   <1.5",
            ] {
                assert_eq!(
                    parse_semver_range(input).unwrap(),
                    Range {
                        clauses: vec![Clause::All(vec![req("^1"), req("<1.5")])]
                    },
                    "input: {input}"
                );
            }

            assert_eq!(
                parse_semver_range(">=1.2.7 <1.3.0").unwrap(),
                Range {
                    clauses: vec![Clause::All(vec![req(">=1.2.7"), req("<1.3.0")])]
                }
            );
        }

        #[test]
        fn parses_or() {
            assert_eq!(
                parse_semver_range("^1 || ^2 || ~3").unwrap(),
                Range {
                    clauses: vec![
                        Clause::Only(req("^1")),
                        Clause::Only(req("^2")),
                        Clause::Only(req("~3")),
                    ]
                }
            );

            assert_eq!(
                parse_semver_range("1||2").unwrap(),
                Range {
                    clauses: vec![Clause::Only(req("1")), Clause::Only(req("2"))]
                }
            );
        }

        #[test]
        fn parses_and_or() {
            assert_eq!(
                parse_semver_range("^1 && <1.5 || >=2, <2.5 || 3.x").unwrap(),
                Range {
                    clauses: vec![
                        Clause::All(vec![req("^1"), req("<1.5")]),
                        Clause::All(vec![req(">=2"), req("<2.5")]),
                        Clause::Only(req("3.x")),
                    ]
                }
            );
        }

        #[test]
        fn parses_full_reqs() {
            // Requirements keep their pre/scope support within ranges
            assert_eq!(
                parse_semver_range(">=1.2.3-alpha && <2.0.0 || node-16-1.2").unwrap(),
                Range {
                    clauses: vec![
                        Clause::All(vec![req(">=1.2.3-alpha"), req("<2.0.0")]),
                        Clause::Only(req("node-16-1.2")),
                    ]
                }
            );

            // Scoped requirements are not limited to the final clause
            assert_eq!(
                parse_semver_range("node-1.2 || node-*").unwrap(),
                Range {
                    clauses: vec![Clause::Only(req("node-1.2")), Clause::Only(req("node-*"))]
                }
            );
        }

        #[test]
        fn parses_and_trims_whitespace() {
            assert_eq!(
                parse_semver_range("  ^1 || ^2  ").unwrap(),
                Range {
                    clauses: vec![Clause::Only(req("^1")), Clause::Only(req("^2"))]
                }
            );
        }

        #[test]
        fn parses_between() {
            for input in ["1.2.3 - 2.3.4", "1.2.3  -  2.3.4"] {
                assert_eq!(
                    parse_semver_range(input).unwrap(),
                    Range {
                        clauses: vec![Clause::Between(ver("1.2.3"), ver("2.3.4"))]
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_between_with_pre_and_build() {
            assert_eq!(
                parse_semver_range("1.2.3-alpha - 2.3.4-beta.1").unwrap(),
                Range {
                    clauses: vec![Clause::Between(ver("1.2.3-alpha"), ver("2.3.4-beta.1"))]
                }
            );

            assert_eq!(
                parse_semver_range("1.2.3+build - 2.3.4").unwrap(),
                Range {
                    clauses: vec![Clause::Between(ver("1.2.3+build"), ver("2.3.4"))]
                }
            );
        }

        #[test]
        fn parses_between_with_or() {
            assert_eq!(
                parse_semver_range("1.2.3 - 2.3.4 || ^3").unwrap(),
                Range {
                    clauses: vec![
                        Clause::Between(ver("1.2.3"), ver("2.3.4")),
                        Clause::Only(req("^3")),
                    ]
                }
            );

            assert_eq!(
                parse_semver_range("^0.5 || 1.2.3 - 2.3.4").unwrap(),
                Range {
                    clauses: vec![
                        Clause::Only(req("^0.5")),
                        Clause::Between(ver("1.2.3"), ver("2.3.4")),
                    ]
                }
            );
        }

        #[test]
        fn parses_v_prefix() {
            // A leading "v" or "V" is ignored
            assert_eq!(
                parse_semver_range("v1 || v1.2.3 - V2.0.0").unwrap(),
                Range {
                    clauses: vec![
                        Clause::Only(req("1")),
                        Clause::Between(ver("1.2.3"), ver("2.0.0")),
                    ]
                }
            );
        }

        #[test]
        fn prefers_pre_over_between() {
            // Without whitespace, the hyphen starts a pre-release
            assert_eq!(
                parse_semver_range("1.2.3-2.3.4").unwrap(),
                Range {
                    clauses: vec![Clause::Only(req("1.2.3-2.3.4"))]
                }
            );
        }

        #[test]
        fn errors_incomplete_clauses() {
            assert!(parse_semver_range("||").is_err());
            assert!(parse_semver_range("^1 ||").is_err());
            assert!(parse_semver_range("|| ^1").is_err());
            assert!(parse_semver_range("^1 &&").is_err());
            assert!(parse_semver_range("&& ^1").is_err());
            assert!(parse_semver_range("^1 && || ^2").is_err());
        }

        #[test]
        fn parses_many_ands() {
            for input in ["^1 && <1.5 && <1.8", "^1, <1.5, <1.8", "^1 <1.5 <1.8"] {
                assert_eq!(
                    parse_semver_range(input).unwrap(),
                    Range {
                        clauses: vec![Clause::All(vec![req("^1"), req("<1.5"), req("<1.8")])]
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn errors_invalid_separators() {
            assert!(parse_semver_range("^1 | ^2").is_err());
            assert!(parse_semver_range("^1 or ^2").is_err());
        }

        #[test]
        fn errors_between_with_partial_versions() {
            // Both sides must be fully qualified
            assert!(parse_semver_range("1.2 - 2.3.4").is_err());
            assert!(parse_semver_range("1.2.3 - 2.3").is_err());
            assert!(parse_semver_range("1 - 2").is_err());
            assert!(parse_semver_range("1.2.3 - x").is_err());
        }

        #[test]
        fn errors_between_with_ops() {
            assert!(parse_semver_range("^1.0.0 - 2.0.0").is_err());
            assert!(parse_semver_range(">=1.2.3 - 2.3.4").is_err());
        }

        #[test]
        fn errors_between_with_and() {
            // A bounded range cannot be combined with an "and"
            assert!(parse_semver_range("^1 && 1.2.3 - 2.0.0").is_err());
            assert!(parse_semver_range("1.2.3 - 2.0.0 && <3").is_err());
        }

        #[test]
        fn errors_between_incomplete() {
            assert!(parse_semver_range("1.2.3 -").is_err());
            assert!(parse_semver_range("- 2.3.4").is_err());
            assert!(parse_semver_range("1.2.3 -2.3.4").is_err());
            assert!(parse_semver_range("1.2.3- 2.3.4").is_err());
        }
    }

    mod calver {
        use super::*;

        #[test]
        fn parses() {
            // Short years are expanded to 4 digits, from the year 2000
            for (input, year, month) in [
                ("2024-02", 2024, 2),
                ("2024-2", 2024, 2),
                ("2024-12", 2024, 12),
                ("224-3", 2224, 3),
                ("24-03", 2024, 3),
                ("04-10", 2004, 10),
            ] {
                assert_eq!(
                    parse_calver(input).unwrap(),
                    Version {
                        kind: VersionKind::Calendar,
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
                        kind: VersionKind::Calendar,
                        major: 2024,
                        minor: 2,
                        patch: day,
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn rejects_dot_format() {
            // Calendar versions use "-" only; a "." separator is semver
            assert!(parse_calver("2024.02").is_err());
            assert!(parse_calver("2024.2.26").is_err());
            assert!(parse_calver("24.12").is_err());
        }

        #[test]
        fn parses_and_trims_whitespace() {
            assert_eq!(
                parse_calver("  2024-02  ").unwrap(),
                Version {
                    kind: VersionKind::Calendar,
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
                    kind: VersionKind::Calendar,
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
                        kind: VersionKind::Calendar,
                        major: 2024,
                        minor: 2,
                        patch: day,
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
                    kind: VersionKind::Calendar,
                    scope: Some("node".into()),
                    major: 2024,
                    minor: 2,
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_calver("foo-bar-2024-5-12").unwrap(),
                Version {
                    kind: VersionKind::Calendar,
                    scope: Some("foo-bar".into()),
                    major: 2024,
                    minor: 5,
                    patch: 12,
                    ..Default::default()
                }
            );

            // short year
            assert_eq!(
                parse_calver("foo_bar-24-1").unwrap(),
                Version {
                    kind: VersionKind::Calendar,
                    scope: Some("foo_bar".into()),
                    major: 2024,
                    minor: 1,
                    ..Default::default()
                }
            );

            // scope segment that looks like a version start
            assert_eq!(
                parse_calver("node-16-2024-02").unwrap(),
                Version {
                    kind: VersionKind::Calendar,
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
                    kind: VersionKind::Calendar,
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
                    kind: VersionKind::Calendar,
                    major: 2024,
                    minor: 5,
                    patch: 1,
                    prerelease: Some("alpha.1".into()),
                    ..Default::default()
                }
            );

            // And with a scope, the longest version match wins
            assert_eq!(
                parse_calver("foo-2024-05-1-alpha.1").unwrap(),
                Version {
                    kind: VersionKind::Calendar,
                    scope: Some("foo".into()),
                    major: 2024,
                    minor: 5,
                    patch: 1,
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
        fn errors_year_too_short() {
            assert!(parse_calver("4-1").is_err());
            assert!(parse_calver("4.1").is_err());
            assert!(parse_calver("0-2-3").is_err());
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
        fn parses_v_prefix() {
            // A leading "v" or "V" is ignored
            for input in ["v2024-02", "V2024-02"] {
                assert_eq!(
                    parse_calver(input).unwrap(),
                    Version {
                        kind: VersionKind::Calendar,
                        major: 2024,
                        minor: 2,
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }
    }

    mod calver_req {
        use super::*;

        #[test]
        fn parses_wildcard() {
            for input in ["", "*", "  *  ", "x", "X"] {
                assert_eq!(
                    parse_calver_req(input).unwrap(),
                    Requirement {
                        kind: VersionKind::Calendar,
                        op: Op::Wildcard,
                        ..Default::default()
                    },
                    "input: {input:?}"
                );
            }
        }

        #[test]
        fn parses_partial() {
            // No operator defaults to a caret match, and short years
            // are expanded to 4 digits, from the year 2000
            for (input, year) in [("2000", 2000), ("224", 2224), ("24", 2024), ("00", 2000)] {
                assert_eq!(
                    parse_calver_req(input).unwrap(),
                    Requirement {
                        kind: VersionKind::Calendar,
                        major: Some(year),
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses() {
            for (input, year, month) in [
                ("2000-2", 2000, 2),
                ("2000-02", 2000, 2),
                ("2000-12", 2000, 12),
                ("224-3", 2224, 3),
                ("24-3", 2024, 3),
                ("04-10", 2004, 10),
            ] {
                assert_eq!(
                    parse_calver_req(input).unwrap(),
                    Requirement {
                        kind: VersionKind::Calendar,
                        major: Some(year),
                        minor: Some(month),
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_day() {
            for (input, day) in [
                ("2024-1-1", 1),
                ("2024-1-09", 9),
                ("2024-1-18", 18),
                ("2024-1-31", 31),
            ] {
                assert_eq!(
                    parse_calver_req(input).unwrap(),
                    Requirement {
                        kind: VersionKind::Calendar,
                        major: Some(2024),
                        minor: Some(1),
                        patch: Some(day),
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn errors_invalid_months() {
            assert!(parse_calver_req("2000-0").is_err());
            assert!(parse_calver_req("2000-00").is_err());
            assert!(parse_calver_req("2000-13").is_err());
            assert!(parse_calver_req("2000-20").is_err());
            assert!(parse_calver_req("2000-2024").is_err());
        }

        #[test]
        fn errors_invalid_days() {
            assert!(parse_calver_req("2000-10-0").is_err());
            assert!(parse_calver_req("2000-10-00").is_err());
            assert!(parse_calver_req("2000-10-32").is_err());
            assert!(parse_calver_req("2000-10-40").is_err());
            assert!(parse_calver_req("2000-2-42").is_err());
            assert!(parse_calver_req("2000-1-32").is_err());
        }

        #[test]
        fn parses_part_wildcards() {
            // A wildcard part matches any value, and marks the
            // whole requirement as a wildcard match
            for (input, month, day) in [
                ("2000-*", None, None),
                ("2000-x", None, None),
                ("2000-X", None, None),
                ("2000-*-*", None, None),
                ("2000-2-*", Some(2), None),
                ("2000-2-x", Some(2), None),
            ] {
                assert_eq!(
                    parse_calver_req(input).unwrap(),
                    Requirement {
                        kind: VersionKind::Calendar,
                        op: Op::Wildcard,
                        major: Some(2000),
                        minor: month,
                        patch: day,
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_ops() {
            for (input, op) in [
                ("=2000-10", Op::Exact),
                ("==2000-10", Op::Exact),
                (">2000-10", Op::Greater),
                (">=2000-10", Op::GreaterEq),
                ("<2000-10", Op::Less),
                ("<=2000-10", Op::LessEq),
                ("~2000-10", Op::Tilde),
                ("^2000-10", Op::Caret),
            ] {
                assert_eq!(
                    parse_calver_req(input).unwrap(),
                    Requirement {
                        kind: VersionKind::Calendar,
                        op,
                        major: Some(2000),
                        minor: Some(10),
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_op_with_whitespace() {
            for (input, op) in [
                ("= 2000-10", Op::Exact),
                ("> 2000-10", Op::Greater),
                (">= 2000-10", Op::GreaterEq),
                ("<  2000-10", Op::Less),
                ("<=  2000-10", Op::LessEq),
                ("~ 2000-10", Op::Tilde),
                ("^ 2000-10", Op::Caret),
            ] {
                assert_eq!(
                    parse_calver_req(input).unwrap(),
                    Requirement {
                        kind: VersionKind::Calendar,
                        op,
                        major: Some(2000),
                        minor: Some(10),
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_ops_with_partial() {
            assert_eq!(
                parse_calver_req(">=2000").unwrap(),
                Requirement {
                    kind: VersionKind::Calendar,
                    op: Op::GreaterEq,
                    major: Some(2000),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_calver_req("~24").unwrap(),
                Requirement {
                    kind: VersionKind::Calendar,
                    op: Op::Tilde,
                    major: Some(2024),
                    ..Default::default()
                }
            );

            // with a wildcard part
            assert_eq!(
                parse_calver_req(">=2000-x").unwrap(),
                Requirement {
                    kind: VersionKind::Calendar,
                    op: Op::GreaterEq,
                    major: Some(2000),
                    ..Default::default()
                }
            );

            // with a dash separator
            assert_eq!(
                parse_calver_req(">=2000-10").unwrap(),
                Requirement {
                    kind: VersionKind::Calendar,
                    op: Op::GreaterEq,
                    major: Some(2000),
                    minor: Some(10),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_pre() {
            assert_eq!(
                parse_calver_req("2000-10-rc.1").unwrap(),
                Requirement {
                    kind: VersionKind::Calendar,
                    major: Some(2000),
                    minor: Some(10),
                    prerelease: Some("rc.1".into()),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_calver_req("2024-2-3-beta.1").unwrap(),
                Requirement {
                    kind: VersionKind::Calendar,
                    major: Some(2024),
                    minor: Some(2),
                    patch: Some(3),
                    prerelease: Some("beta.1".into()),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_calver_req(">=2024-2-alpha").unwrap(),
                Requirement {
                    kind: VersionKind::Calendar,
                    op: Op::GreaterEq,
                    major: Some(2024),
                    minor: Some(2),
                    prerelease: Some("alpha".into()),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_and_trims_whitespace() {
            assert_eq!(
                parse_calver_req("  >= 2000-10  ").unwrap(),
                Requirement {
                    kind: VersionKind::Calendar,
                    op: Op::GreaterEq,
                    major: Some(2000),
                    minor: Some(10),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn errors_missing_version() {
            assert!(parse_calver_req(">=").is_err());
            assert!(parse_calver_req("~").is_err());
            assert!(parse_calver_req("^").is_err());
            assert!(parse_calver_req("2000-").is_err());
            assert!(parse_calver_req("node-").is_err());
        }

        #[test]
        fn errors_invalid_ops() {
            assert!(parse_calver_req("=>2000").is_err());
            assert!(parse_calver_req(">>2000").is_err());
            assert!(parse_calver_req("!=2000").is_err());
        }

        #[test]
        fn errors_interior_whitespace() {
            // Whitespace is only allowed after the operator
            assert!(parse_calver_req("2000 -2").is_err());
            assert!(parse_calver_req("2000- 2").is_err());
            assert!(parse_calver_req("> =2000").is_err());
        }

        #[test]
        fn errors_too_many_parts() {
            assert!(parse_calver_req("2000-1-2-3").is_err());
        }

        #[test]
        fn errors_year_too_long() {
            assert!(parse_calver_req("20000").is_err());
            assert!(parse_calver_req("20000.2").is_err());
        }

        #[test]
        fn errors_year_too_short() {
            assert!(parse_calver_req("0").is_err());
            assert!(parse_calver_req("4").is_err());
            assert!(parse_calver_req("4.1").is_err());
            assert!(parse_calver_req("4-1").is_err());
        }

        #[test]
        fn parses_and_ignores_build_metadata() {
            assert_eq!(
                parse_calver_req("2000-2+build").unwrap(),
                parse_calver_req("2000-2").unwrap()
            );
            assert_eq!(
                parse_calver_req(">=2024-2-alpha+build.5").unwrap(),
                parse_calver_req(">=2024-2-alpha").unwrap()
            );
        }

        #[test]
        fn errors_digit_pre() {
            // A calver pre-release must start with a letter
            assert!(parse_calver_req("2000-10-0").is_err());
        }

        #[test]
        fn parses_scope() {
            for (input, scope, year, month, day) in [
                ("node-2024-2", "node", 2024, Some(2), None),
                ("node-2024", "node", 2024, None, None),
                ("foo-bar-2024-5-12", "foo-bar", 2024, Some(5), Some(12)),
                ("foo_bar-24-1", "foo_bar", 2024, Some(1), None),
                ("node-16-2024-2", "node-16", 2024, Some(2), None),
            ] {
                assert_eq!(
                    parse_calver_req(input).unwrap(),
                    Requirement {
                        kind: VersionKind::Calendar,
                        scope: Some(scope.into()),
                        major: Some(year),
                        minor: month,
                        patch: day,
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_scope_with_trailing_dash() {
            assert_eq!(
                parse_calver_req("foo--2024-2").unwrap(),
                Requirement {
                    kind: VersionKind::Calendar,
                    scope: Some("foo-".into()),
                    major: Some(2024),
                    minor: Some(2),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_scope_with_op() {
            assert_eq!(
                parse_calver_req(">=node-2024-2").unwrap(),
                Requirement {
                    kind: VersionKind::Calendar,
                    op: Op::GreaterEq,
                    scope: Some("node".into()),
                    major: Some(2024),
                    minor: Some(2),
                    ..Default::default()
                }
            );

            assert_eq!(
                parse_calver_req("~ node-24").unwrap(),
                Requirement {
                    kind: VersionKind::Calendar,
                    op: Op::Tilde,
                    scope: Some("node".into()),
                    major: Some(2024),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_scope_with_pre() {
            assert_eq!(
                parse_calver_req("temurin-2024-1-rc.2").unwrap(),
                Requirement {
                    kind: VersionKind::Calendar,
                    scope: Some("temurin".into()),
                    major: Some(2024),
                    minor: Some(1),
                    prerelease: Some("rc.2".into()),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn parses_scope_with_wildcard() {
            for input in ["node-*", "node-x", "node-X"] {
                assert_eq!(
                    parse_calver_req(input).unwrap(),
                    Requirement {
                        kind: VersionKind::Calendar,
                        op: Op::Wildcard,
                        scope: Some("node".into()),
                        ..Default::default()
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn parses_v_prefix() {
            // A leading "v" or "V" is ignored
            assert_eq!(
                parse_calver_req("v2000-2").unwrap(),
                parse_calver_req("2000-2").unwrap()
            );
            assert_eq!(
                parse_calver_req(">=V2000-2").unwrap(),
                parse_calver_req(">=2000-2").unwrap()
            );
        }

        #[test]
        fn errors_multiple_reqs() {
            // Comma/space separated requirement lists are split upstream
            assert!(parse_calver_req("2000-2, 2001-2").is_err());
            assert!(parse_calver_req(">=2000-1 <2001-1").is_err());
        }
    }

    mod calver_range {
        use super::*;

        fn req(input: &str) -> Requirement {
            parse_calver_req(input).unwrap()
        }

        fn ver(input: &str) -> Box<Version> {
            Box::new(parse_calver(input).unwrap())
        }

        #[test]
        fn parses_single() {
            assert_eq!(
                parse_calver_range("~2000-2").unwrap(),
                Range {
                    clauses: vec![Clause::Only(Requirement {
                        kind: VersionKind::Calendar,
                        op: Op::Tilde,
                        major: Some(2000),
                        minor: Some(2),
                        ..Default::default()
                    })]
                }
            );

            assert_eq!(
                parse_calver_range("2024-1-15").unwrap(),
                Range {
                    clauses: vec![Clause::Only(req("2024-1-15"))]
                }
            );
        }

        #[test]
        fn parses_wildcard() {
            for input in ["", "*", "  *  ", "x", "X"] {
                assert_eq!(
                    parse_calver_range(input).unwrap(),
                    Range::default(),
                    "input: {input:?}"
                );
            }
        }

        #[test]
        fn parses_and() {
            // " ", "&&", and "," are supported
            for input in [
                "2000-2 && 2001-3",
                "2000-2, 2001-3",
                "2000-2,2001-3",
                "2000-2&&2001-3",
                "2000-2 2001-3",
                "2000-2  2001-3",
            ] {
                assert_eq!(
                    parse_calver_range(input).unwrap(),
                    Range {
                        clauses: vec![Clause::All(vec![req("2000-2"), req("2001-3")])]
                    },
                    "input: {input}"
                );
            }

            assert_eq!(
                parse_calver_range(">=2000-1 <2001-1").unwrap(),
                Range {
                    clauses: vec![Clause::All(vec![req(">=2000-1"), req("<2001-1")])]
                }
            );
        }

        #[test]
        fn parses_or() {
            assert_eq!(
                parse_calver_range("2000 || 2001 || 2002").unwrap(),
                Range {
                    clauses: vec![
                        Clause::Only(req("2000")),
                        Clause::Only(req("2001")),
                        Clause::Only(req("2002")),
                    ]
                }
            );

            assert_eq!(
                parse_calver_range("2000-2||2001-3").unwrap(),
                Range {
                    clauses: vec![Clause::Only(req("2000-2")), Clause::Only(req("2001-3"))]
                }
            );
        }

        #[test]
        fn parses_and_or() {
            assert_eq!(
                parse_calver_range(">=2000-1 && <2000-6 || >=2001, <2002 || 2003-x").unwrap(),
                Range {
                    clauses: vec![
                        Clause::All(vec![req(">=2000-1"), req("<2000-6")]),
                        Clause::All(vec![req(">=2001"), req("<2002")]),
                        Clause::Only(req("2003-x")),
                    ]
                }
            );
        }

        #[test]
        fn parses_full_reqs() {
            // Requirements keep their pre/scope support within ranges
            assert_eq!(
                parse_calver_range(">=2000-1-alpha && <2001-1 || node-16-2024-2").unwrap(),
                Range {
                    clauses: vec![
                        Clause::All(vec![req(">=2000-1-alpha"), req("<2001-1")]),
                        Clause::Only(req("node-16-2024-2")),
                    ]
                }
            );

            // Scoped requirements are not limited to the final clause
            assert_eq!(
                parse_calver_range("node-2000-2 || node-*").unwrap(),
                Range {
                    clauses: vec![
                        Clause::Only(req("node-2000-2")),
                        Clause::Only(req("node-*"))
                    ]
                }
            );
        }

        #[test]
        fn parses_and_trims_whitespace() {
            assert_eq!(
                parse_calver_range("  2000 || 2001  ").unwrap(),
                Range {
                    clauses: vec![Clause::Only(req("2000")), Clause::Only(req("2001"))]
                }
            );
        }

        #[test]
        fn parses_between() {
            for input in ["2000-2 - 2001-3", "2000-2  -  2001-3"] {
                assert_eq!(
                    parse_calver_range(input).unwrap(),
                    Range {
                        clauses: vec![Clause::Between(ver("2000-2"), ver("2001-3"))]
                    },
                    "input: {input}"
                );
            }

            // with days
            assert_eq!(
                parse_calver_range("2024-1-15 - 2024-6-30").unwrap(),
                Range {
                    clauses: vec![Clause::Between(ver("2024-1-15"), ver("2024-6-30"))]
                }
            );
        }

        #[test]
        fn parses_between_with_pre() {
            assert_eq!(
                parse_calver_range("2000-2-alpha - 2001-3-beta.1").unwrap(),
                Range {
                    clauses: vec![Clause::Between(ver("2000-2-alpha"), ver("2001-3-beta.1"))]
                }
            );
        }

        #[test]
        fn parses_between_with_or() {
            assert_eq!(
                parse_calver_range("2000-2 - 2001-3 || >=2002").unwrap(),
                Range {
                    clauses: vec![
                        Clause::Between(ver("2000-2"), ver("2001-3")),
                        Clause::Only(req(">=2002")),
                    ]
                }
            );

            assert_eq!(
                parse_calver_range("~2000 || 2000-2 - 2001-3").unwrap(),
                Range {
                    clauses: vec![
                        Clause::Only(req("~2000")),
                        Clause::Between(ver("2000-2"), ver("2001-3")),
                    ]
                }
            );
        }

        #[test]
        fn parses_v_prefix() {
            // A leading "v" or "V" is ignored
            assert_eq!(
                parse_calver_range("v2000 || v2000-2 - V2001-3").unwrap(),
                Range {
                    clauses: vec![
                        Clause::Only(req("2000")),
                        Clause::Between(ver("2000-2"), ver("2001-3")),
                    ]
                }
            );
        }

        #[test]
        fn prefers_version_over_between() {
            // Without whitespace, hyphens are version separators
            assert_eq!(
                parse_calver_range("2000-2-3").unwrap(),
                Range {
                    clauses: vec![Clause::Only(req("2000-2-3"))]
                }
            );
        }

        #[test]
        fn errors_incomplete_clauses() {
            assert!(parse_calver_range("||").is_err());
            assert!(parse_calver_range("2000 ||").is_err());
            assert!(parse_calver_range("|| 2000").is_err());
            assert!(parse_calver_range("2000 &&").is_err());
            assert!(parse_calver_range("&& 2000").is_err());
            assert!(parse_calver_range("2000 && || 2001").is_err());
        }

        #[test]
        fn parses_many_ands() {
            for input in ["2000 && 2001 && 2002", "2000, 2001, 2002", "2000 2001 2002"] {
                assert_eq!(
                    parse_calver_range(input).unwrap(),
                    Range {
                        clauses: vec![Clause::All(vec![req("2000"), req("2001"), req("2002")])]
                    },
                    "input: {input}"
                );
            }
        }

        #[test]
        fn errors_invalid_separators() {
            assert!(parse_calver_range("2000 | 2001").is_err());
            assert!(parse_calver_range("2000 or 2001").is_err());
        }

        #[test]
        fn errors_between_with_partial_versions() {
            // Both sides must be fully qualified, requiring at least a month
            assert!(parse_calver_range("2000 - 2001").is_err());
            assert!(parse_calver_range("2000-2 - 2001").is_err());
            assert!(parse_calver_range("2000 - 2001-2").is_err());
            assert!(parse_calver_range("2000-2 - x").is_err());
        }

        #[test]
        fn errors_between_with_ops() {
            assert!(parse_calver_range("^2000-1 - 2001-1").is_err());
            assert!(parse_calver_range(">=2000-2 - 2001-3").is_err());
        }

        #[test]
        fn errors_between_with_and() {
            // A bounded range cannot be combined with an "and"
            assert!(parse_calver_range("2000-1 && 2000-2 - 2001-1").is_err());
            assert!(parse_calver_range("2000-2 - 2001-1 && <2002-1").is_err());
        }

        #[test]
        fn errors_between_incomplete() {
            assert!(parse_calver_range("2000-2 -").is_err());
            assert!(parse_calver_range("- 2001-3").is_err());
            assert!(parse_calver_range("2000-2 -2001-3").is_err());
            assert!(parse_calver_range("2000-2- 2001-3").is_err());
        }
    }

    mod serde {
        use super::*;

        #[test]
        fn version_to_from_string() {
            for value in [
                "1.2.3",
                "node-1.2.3-alpha.1+build.5",
                "2024-02",
                "2024-02-26",
                "node-2024-02-alpha",
            ] {
                let version = Version::parse(value).unwrap();

                assert_eq!(
                    serde_json::to_string(&version).unwrap(),
                    format!("\"{value}\""),
                    "input: {value}"
                );
                assert_eq!(
                    serde_json::from_str::<Version>(&format!("\"{value}\"")).unwrap(),
                    version,
                    "input: {value}"
                );
            }
        }

        #[test]
        fn requirement_to_from_string() {
            for value in [
                "*",
                "=1",
                "=1.2.3",
                "1.*",
                ">=1.2",
                "~1",
                "^1.2.3-beta.1",
                "node-*",
                "=node-1.2",
                "=2000-02",
                ">=2000-02-03",
            ] {
                let req = Requirement::parse(value).unwrap();

                assert_eq!(
                    serde_json::to_string(&req).unwrap(),
                    format!("\"{value}\""),
                    "input: {value}"
                );
                assert_eq!(
                    serde_json::from_str::<Requirement>(&format!("\"{value}\"")).unwrap(),
                    req,
                    "input: {value}"
                );
            }
        }

        #[test]
        fn range_to_from_string() {
            for value in [
                "*",
                "=1.2.3",
                "^1 && <1.5",
                "^1 || ^2 || ~3",
                "1.2.3 - 2.3.4",
                "=2000-02 || =2001-03",
                "2000-02 - 2001-03",
            ] {
                let range = Range::parse(value).unwrap();

                assert_eq!(
                    serde_json::to_string(&range).unwrap(),
                    format!("\"{value}\""),
                    "input: {value}"
                );
                assert_eq!(
                    serde_json::from_str::<Range>(&format!("\"{value}\"")).unwrap(),
                    range,
                    "input: {value}"
                );
            }
        }

        #[test]
        fn serializes_normalized() {
            // Short years, zero-padding, wildcard parts, separators,
            // and v prefixes are normalized
            let version = Version::parse("24-1").unwrap();

            assert_eq!(serde_json::to_string(&version).unwrap(), "\"2024-01\"");

            let version = Version::parse("v1.2.3").unwrap();

            assert_eq!(serde_json::to_string(&version).unwrap(), "\"1.2.3\"");

            // Calendar month and day are zero-padded
            let version = Version::parse("2024-2-26").unwrap();

            assert_eq!(serde_json::to_string(&version).unwrap(), "\"2024-02-26\"");

            let req = Requirement::parse("1.x").unwrap();

            assert_eq!(serde_json::to_string(&req).unwrap(), "\"1.*\"");

            let req = Requirement::parse("=1.2.3+build").unwrap();

            assert_eq!(serde_json::to_string(&req).unwrap(), "\"=1.2.3\"");

            let range = Range::parse("^1, <1.5").unwrap();

            assert_eq!(serde_json::to_string(&range).unwrap(), "\"^1 && <1.5\"");

            let range = Range::parse("").unwrap();

            assert_eq!(serde_json::to_string(&range).unwrap(), "\"*\"");
        }
    }

    mod ordering {
        use super::*;

        fn sorted<const N: usize>(inputs: [&str; N]) -> Vec<String> {
            let mut versions = inputs.map(|input| Version::parse(input).unwrap());
            versions.sort();
            versions.iter().map(ToString::to_string).collect()
        }

        fn req(input: &str) -> Requirement {
            Requirement::parse(input).unwrap()
        }

        fn range(input: &str) -> Range {
            Range::parse(input).unwrap()
        }

        #[test]
        fn orders_versions() {
            assert_eq!(
                sorted(["10.0.0", "1.2.4", "2.0.0", "1.2.3", "1.10.0", "1.3.0"]),
                ["1.2.3", "1.2.4", "1.3.0", "1.10.0", "2.0.0", "10.0.0"]
            );
        }

        #[test]
        fn orders_prereleases() {
            // The example ordering from the semver spec
            assert_eq!(
                sorted([
                    "1.0.0-beta.11",
                    "1.0.0",
                    "1.0.0-alpha.beta",
                    "1.0.0-rc.1",
                    "1.0.0-alpha.1",
                    "1.0.0-beta",
                    "1.0.0-alpha",
                    "1.0.0-beta.2",
                ]),
                [
                    "1.0.0-alpha",
                    "1.0.0-alpha.1",
                    "1.0.0-alpha.beta",
                    "1.0.0-beta",
                    "1.0.0-beta.2",
                    "1.0.0-beta.11",
                    "1.0.0-rc.1",
                    "1.0.0",
                ]
            );
        }

        #[test]
        fn orders_build_metadata() {
            // None sorts first, then numeric identifiers including
            // leading zeros, then alphanumeric identifiers
            assert_eq!(
                sorted([
                    "1.0.0+01",
                    "1.0.0+00",
                    "1.0.0+alpha",
                    "1.0.0",
                    "1.0.0+1",
                    "1.0.0+0",
                    "1.0.0+2",
                ]),
                [
                    "1.0.0",
                    "1.0.0+0",
                    "1.0.0+00",
                    "1.0.0+1",
                    "1.0.0+01",
                    "1.0.0+2",
                    "1.0.0+alpha",
                ]
            );
        }

        #[test]
        fn orders_calendar_versions() {
            assert_eq!(
                sorted(["2024-2", "2024-1-15", "2023-12-31", "2024-1"]),
                ["2023-12-31", "2024-01", "2024-01-15", "2024-02"]
            );
        }

        #[test]
        fn orders_kinds_and_scopes() {
            let ver = |input: &str| Version::parse(input).unwrap();

            // Calendar versions group before semantic versions
            assert!(ver("2024-2") < ver("1.0.0"));

            // Unscoped versions group before scoped versions,
            // and scopes group before version numbers
            assert!(ver("1.0.0") < ver("aaa-1.0.0"));
            assert!(ver("aaa-2.0.0") < ver("bbb-1.0.0"));
        }

        #[test]
        fn orders_requirements() {
            // Wildcard parts order first
            assert!(req("*") < req("=1"));
            assert!(req("=1.x") < req("=1.2"));

            // Then by version, with the operator as the tiebreaker
            assert!(req("=1.2") < req("=1.3"));
            assert!(req("=1.2.3-alpha") < req("=1.2.3"));
            assert!(req(">1.2") < req(">=1.2"));

            // Calendar requirements
            assert!(req("=2000-2") < req("=2000-3"));
        }

        #[test]
        fn orders_ranges() {
            // Clauses are compared in order, with an empty range first
            assert!(range("") < range("=1"));
            assert!(range("=1") < range("=1 || =2"));
            assert!(range("=1 || =2") < range("=2"));

            // "all" clauses order before "between", then "only" clauses
            assert!(range("=1 && =2") < range("1.2.3 - 2.3.4"));
            assert!(range("1.2.3 - 2.3.4") < range("=1"));
        }
    }

    mod matches {
        use super::*;

        fn ver(input: &str) -> Version {
            Version::parse(input).unwrap()
        }

        fn req(input: &str) -> Requirement {
            Requirement::parse(input).unwrap()
        }

        fn range(input: &str) -> Range {
            Range::parse(input).unwrap()
        }

        #[test]
        fn matches_wildcard() {
            assert!(req("*").matches(&ver("1.2.3")));
            assert!(req("*").matches(&ver("2024-2")));

            // Except pre-releases
            assert!(!req("*").matches(&ver("1.2.3-alpha")));

            // Same for an empty range
            assert!(range("").matches(&ver("1.2.3")));
            assert!(!range("").matches(&ver("1.2.3-alpha")));
        }

        #[test]
        fn matches_exact() {
            assert!(req("=1.2.3").matches(&ver("1.2.3")));
            assert!(!req("=1.2.3").matches(&ver("1.2.4")));

            // A partial matches any omitted part
            assert!(req("=1.2").matches(&ver("1.2.9")));
            assert!(!req("=1.2").matches(&ver("1.3.0")));

            // Build metadata is ignored
            assert!(req("=1.2.3").matches(&ver("1.2.3+build")));
        }

        #[test]
        fn matches_greater_and_less() {
            assert!(req(">1.2.3").matches(&ver("1.2.4")));
            assert!(!req(">1.2.3").matches(&ver("1.2.3")));

            // A partial only matches beyond the specified parts
            assert!(req(">1").matches(&ver("2.0.0")));
            assert!(!req(">1").matches(&ver("1.5.0")));

            assert!(req(">=1.2").matches(&ver("1.2.0")));
            assert!(req("<2").matches(&ver("1.9.9")));
            assert!(!req("<2").matches(&ver("2.0.0")));
            assert!(req("<=1.2").matches(&ver("1.2.9")));
        }

        #[test]
        fn matches_tilde() {
            assert!(req("~1.2.3").matches(&ver("1.2.9")));
            assert!(!req("~1.2.3").matches(&ver("1.2.2")));
            assert!(!req("~1.2.3").matches(&ver("1.3.0")));

            assert!(req("~1.2").matches(&ver("1.2.9")));
            assert!(!req("~1.2").matches(&ver("1.3.0")));

            assert!(req("~1").matches(&ver("1.9.0")));
            assert!(!req("~1").matches(&ver("2.0.0")));
        }

        #[test]
        fn matches_caret() {
            assert!(req("^1.2.3").matches(&ver("1.9.9")));
            assert!(!req("^1.2.3").matches(&ver("1.2.2")));
            assert!(!req("^1.2.3").matches(&ver("2.0.0")));

            // A zero major locks the minor
            assert!(req("^0.2.3").matches(&ver("0.2.9")));
            assert!(!req("^0.2.3").matches(&ver("0.3.0")));

            // A zero major and minor locks the micro
            assert!(req("^0.0.3").matches(&ver("0.0.3")));
            assert!(!req("^0.0.3").matches(&ver("0.0.4")));

            assert!(req("^0").matches(&ver("0.9.9")));
            assert!(!req("^0").matches(&ver("1.0.0")));
        }

        #[test]
        fn matches_prereleases() {
            // A pre-release only matches when the requirement has a
            // pre-release on the same version numbers
            assert!(req(">=1.2.3-alpha").matches(&ver("1.2.3-beta")));
            assert!(req(">=1.2.3-alpha").matches(&ver("1.2.3")));
            assert!(req(">=1.2.3-alpha").matches(&ver("1.2.4")));
            assert!(!req(">=1.2.3-alpha").matches(&ver("1.2.4-beta")));

            assert!(req("=1.2.3-alpha").matches(&ver("1.2.3-alpha")));
            assert!(!req("=1.2.3").matches(&ver("1.2.3-alpha")));
        }

        #[test]
        fn matches_scopes() {
            // A scoped requirement only matches the same scope
            assert!(req("node-*").matches(&ver("node-1.2.3")));
            assert!(!req("node-*").matches(&ver("1.2.3")));
            assert!(!req("node-*").matches(&ver("bun-1.2.3")));

            // An unscoped requirement matches any scope
            assert!(req("^1").matches(&ver("node-1.2.3")));
        }

        #[test]
        fn matches_calver() {
            assert!(req("~2024-2").matches(&ver("2024-2-15")));
            assert!(!req("~2024-2").matches(&ver("2024-3-1")));

            assert!(req("=2024-2").matches(&ver("2024-2-9")));
            assert!(req(">=2024-6").matches(&ver("2024-6-1")));
            assert!(!req(">=2024-6").matches(&ver("2024-5-31")));

            // The kind is ignored when matching
            assert!(req("~2024.2").matches(&ver("2024-2-15")));
        }

        #[test]
        fn matches_and_clauses() {
            let and = range(">=1.2.3-alpha && <2");

            // Pre-release compatibility is satisfied clause-wide
            assert!(and.matches(&ver("1.2.3-alpha")));
            assert!(and.matches(&ver("1.5.0")));
            assert!(!and.matches(&ver("1.2.2")));
            assert!(!and.matches(&ver("2.0.0")));
        }

        #[test]
        fn matches_between_clauses() {
            let between = range("1.2.3 - 2.3.4");

            // Inclusive on both ends
            assert!(between.matches(&ver("1.2.3")));
            assert!(between.matches(&ver("2.0.0")));
            assert!(between.matches(&ver("2.3.4")));
            assert!(!between.matches(&ver("1.2.2")));
            assert!(!between.matches(&ver("2.3.5")));
            assert!(!between.matches(&ver("1.5.0-beta")));

            // Unless a bound has a pre-release on the same version
            assert!(range("1.2.3-alpha - 2.0.0").matches(&ver("1.2.3-beta")));
        }

        #[test]
        fn matches_between_calver_clauses() {
            let between = range("2000-2 - 2001-3");

            // A day-less bound is month-granular
            assert!(between.matches(&ver("2000-2-1")));
            assert!(between.matches(&ver("2001-3-15")));
            assert!(!between.matches(&ver("2000-1-31")));
            assert!(!between.matches(&ver("2001-4-1")));
        }

        #[test]
        fn matches_or_clauses() {
            let or = range("^1 || ^2");

            assert!(or.matches(&ver("1.5.0")));
            assert!(or.matches(&ver("2.5.0")));
            assert!(!or.matches(&ver("3.0.0")));
        }
    }
}
