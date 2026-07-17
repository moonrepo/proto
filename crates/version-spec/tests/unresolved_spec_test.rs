use compact_str::CompactString;
use version_spec::{Clause, Op, Range, Requirement, UnresolvedVersionSpec, Version, VersionKind};

fn req(input: &str) -> Requirement {
    Requirement::parse(input).unwrap()
}

mod unresolved_spec {
    use super::*;

    #[test]
    fn canary() {
        assert_eq!(
            UnresolvedVersionSpec::parse("canary").unwrap(),
            UnresolvedVersionSpec::Canary
        );
    }

    #[test]
    fn aliases() {
        assert_eq!(
            UnresolvedVersionSpec::parse("latest").unwrap(),
            UnresolvedVersionSpec::Alias(CompactString::new("latest"))
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("stable").unwrap(),
            UnresolvedVersionSpec::Alias(CompactString::new("stable"))
        );
        // A dashed alias remains an alias when the tail is not version-like
        assert_eq!(
            UnresolvedVersionSpec::parse("lts-hydrogen").unwrap(),
            UnresolvedVersionSpec::Alias(CompactString::new("lts-hydrogen"))
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("future/202x").unwrap(),
            UnresolvedVersionSpec::Alias(CompactString::new("future/202x"))
        );
    }

    #[test]
    fn versions() {
        assert_eq!(
            UnresolvedVersionSpec::parse("v1.2.3").unwrap(),
            UnresolvedVersionSpec::Version(Version::semantic(1, 2, 3))
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2.3").unwrap(),
            UnresolvedVersionSpec::Version(Version::semantic(1, 2, 3))
        );

        // calver, in which a year-month is fully-qualified
        assert_eq!(
            UnresolvedVersionSpec::parse("2024-02").unwrap(),
            UnresolvedVersionSpec::Version(Version {
                kind: VersionKind::Calendar,
                major: 2024,
                minor: 2,
                ..Default::default()
            })
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("2024-2-26").unwrap(),
            UnresolvedVersionSpec::Version(Version::calendar(2024, 2, 26))
        );
    }

    #[test]
    fn scoped_versions() {
        assert_eq!(
            UnresolvedVersionSpec::parse("node-1.2.3").unwrap(),
            UnresolvedVersionSpec::Version(Version {
                scope: Some("node".into()),
                ..Version::semantic(1, 2, 3)
            })
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("node-1.2.3-alpha.1").unwrap(),
            UnresolvedVersionSpec::Version(Version {
                scope: Some("node".into()),
                prerelease: Some("alpha.1".into()),
                ..Version::semantic(1, 2, 3)
            })
        );

        // calver
        assert_eq!(
            UnresolvedVersionSpec::parse("node-2024-02").unwrap(),
            UnresolvedVersionSpec::Version(Version {
                kind: VersionKind::Calendar,
                scope: Some("node".into()),
                major: 2024,
                minor: 2,
                ..Default::default()
            })
        );

        // A scoped partial is a requirement, not a version
        assert_eq!(
            UnresolvedVersionSpec::parse("temurin-21").unwrap(),
            UnresolvedVersionSpec::Requirement(Requirement {
                scope: Some("temurin".into()),
                major: Some(21),
                ..Default::default()
            })
        );
    }

    #[test]
    fn serde_roundtrip() {
        for value in [
            "canary",
            "latest",
            "lts-2014",
            "^1.2",
            "1.2.3",
            "node-1.2.3",
            "2024-2-26",
            "node-2024-02",
        ] {
            let spec = UnresolvedVersionSpec::parse(value).unwrap();
            let json = serde_json::to_string(&spec).unwrap();

            assert_eq!(
                serde_json::from_str::<UnresolvedVersionSpec>(&json).unwrap(),
                spec
            );
        }
    }

    #[test]
    fn requirements() {
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2").unwrap(),
            UnresolvedVersionSpec::Requirement(Requirement {
                major: Some(1),
                minor: Some(2),
                ..Default::default()
            })
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("~2000-2").unwrap(),
            UnresolvedVersionSpec::Requirement(Requirement {
                kind: VersionKind::Calendar,
                op: Op::Tilde,
                major: Some(2000),
                minor: Some(2),
                ..Default::default()
            })
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1").unwrap(),
            UnresolvedVersionSpec::Requirement(Requirement {
                major: Some(1),
                ..Default::default()
            })
        );

        // a year alone is not calver-like, so parses as semantic
        assert_eq!(
            UnresolvedVersionSpec::parse("2000").unwrap(),
            UnresolvedVersionSpec::Requirement(Requirement {
                major: Some(2000),
                ..Default::default()
            })
        );

        assert_eq!(
            UnresolvedVersionSpec::parse(">1").unwrap(),
            UnresolvedVersionSpec::Requirement(Requirement {
                op: Op::Greater,
                major: Some(1),
                ..Default::default()
            })
        );
        assert_eq!(
            UnresolvedVersionSpec::parse(">2000-10").unwrap(),
            UnresolvedVersionSpec::Requirement(Requirement {
                kind: VersionKind::Calendar,
                op: Op::Greater,
                major: Some(2000),
                minor: Some(10),
                ..Default::default()
            })
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("<=1").unwrap(),
            UnresolvedVersionSpec::Requirement(Requirement {
                op: Op::LessEq,
                major: Some(1),
                ..Default::default()
            })
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("<=2000-12-12").unwrap(),
            UnresolvedVersionSpec::Requirement(Requirement {
                kind: VersionKind::Calendar,
                op: Op::LessEq,
                major: Some(2000),
                minor: Some(12),
                patch: Some(12),
                ..Default::default()
            })
        );
    }

    #[test]
    fn wildcard_requirements() {
        for (input, major, minor) in [
            ("1.2.*", Some(1), Some(2)),
            ("1.2.X", Some(1), Some(2)),
            ("1.*", Some(1), None),
            ("1.x", Some(1), None),
            ("1.x.x", Some(1), None),
        ] {
            assert_eq!(
                UnresolvedVersionSpec::parse(input).unwrap(),
                UnresolvedVersionSpec::Requirement(Requirement {
                    op: Op::Wildcard,
                    major,
                    minor,
                    ..Default::default()
                }),
                "input: {input}"
            );
        }

        // dashed date-like parts parse as calendar
        assert_eq!(
            UnresolvedVersionSpec::parse("2000-02-*").unwrap(),
            UnresolvedVersionSpec::Requirement(Requirement {
                kind: VersionKind::Calendar,
                op: Op::Wildcard,
                major: Some(2000),
                minor: Some(2),
                ..Default::default()
            })
        );
    }

    #[test]
    fn requirement_lists() {
        // comma and space separated lists become a range clause
        for input in ["1, 2", "1,2", "1 2"] {
            assert_eq!(
                UnresolvedVersionSpec::parse(input).unwrap(),
                UnresolvedVersionSpec::Range(Range {
                    clauses: vec![Clause::All(vec![req("1"), req("2")])]
                }),
                "input: {input}"
            );
        }

        assert_eq!(
            UnresolvedVersionSpec::parse("2000-05, 3000-01").unwrap(),
            UnresolvedVersionSpec::Range(Range {
                clauses: vec![Clause::All(vec![req("2000-05"), req("3000-01")])]
            })
        );
    }

    #[test]
    fn any_requirements() {
        assert_eq!(
            UnresolvedVersionSpec::parse("^1.2 || ~1 || 3,4").unwrap(),
            UnresolvedVersionSpec::Range(Range {
                clauses: vec![
                    Clause::Only(req("^1.2")),
                    Clause::Only(req("~1")),
                    Clause::All(vec![req("3"), req("4")]),
                ]
            })
        );

        assert_eq!(
            UnresolvedVersionSpec::parse("^2000-10 || ~1000 || 3000-05-12,4000-09-09").unwrap(),
            UnresolvedVersionSpec::Range(Range {
                clauses: vec![
                    Clause::Only(req("^2000-10")),
                    // Inherits the calendar kind from the range
                    Clause::Only(Requirement {
                        kind: VersionKind::Calendar,
                        op: Op::Tilde,
                        major: Some(1000),
                        ..Default::default()
                    }),
                    Clause::All(vec![req("3000-05-12"), req("4000-09-09")]),
                ]
            })
        );
    }

    #[test]
    fn parses_alias() {
        assert_eq!(
            UnresolvedVersionSpec::parse("stable").unwrap(),
            UnresolvedVersionSpec::Alias(CompactString::new("stable"))
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("latest").unwrap(),
            UnresolvedVersionSpec::Alias(CompactString::new("latest"))
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("lts-hydrogen").unwrap(),
            UnresolvedVersionSpec::Alias(CompactString::new("lts-hydrogen"))
        );
    }

    #[test]
    fn parses_req() {
        for input in ["=1.2.3", "^1.2", "~1", ">1.2.0", "<1", "*"] {
            assert_eq!(
                UnresolvedVersionSpec::parse(input).unwrap(),
                UnresolvedVersionSpec::Requirement(req(input)),
                "input: {input}"
            );
        }

        // multiple values become a range clause
        assert_eq!(
            UnresolvedVersionSpec::parse(">1, <=1.5").unwrap(),
            UnresolvedVersionSpec::Range(Range {
                clauses: vec![Clause::All(vec![req(">1"), req("<=1.5")])]
            })
        );
    }

    #[test]
    fn parses_req_spaces() {
        // A space after the operator is still a single requirement
        assert_eq!(
            UnresolvedVersionSpec::parse("> 10").unwrap(),
            UnresolvedVersionSpec::Requirement(req(">10"))
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2 , 2").unwrap(),
            UnresolvedVersionSpec::Range(Range {
                clauses: vec![Clause::All(vec![req("1.2"), req("2")])]
            })
        );
        assert_eq!(
            UnresolvedVersionSpec::parse(">= 1.2 < 2").unwrap(),
            UnresolvedVersionSpec::Range(Range {
                clauses: vec![Clause::All(vec![req(">=1.2"), req("<2")])]
            })
        );
    }

    #[test]
    fn parses_req_any() {
        // clause order is preserved
        assert_eq!(
            UnresolvedVersionSpec::parse("^1 || ~2 || =3").unwrap(),
            UnresolvedVersionSpec::Range(Range {
                clauses: vec![
                    Clause::Only(req("^1")),
                    Clause::Only(req("~2")),
                    Clause::Only(req("=3")),
                ]
            })
        );
    }

    #[test]
    fn parses_version() {
        for input in ["1.2.3", "4.5.6", "7.8.9-alpha", "10.20.30+build"] {
            assert_eq!(
                UnresolvedVersionSpec::parse(input).unwrap(),
                UnresolvedVersionSpec::Version(Version::parse(input).unwrap()),
                "input: {input}"
            );
        }

        // a dotted date-like version parses as semver, not calendar
        assert_eq!(
            UnresolvedVersionSpec::parse("10.11.12").unwrap(),
            UnresolvedVersionSpec::Version(Version::semantic(10, 11, 12))
        );
    }

    #[test]
    fn parses_version_with_v() {
        assert_eq!(
            UnresolvedVersionSpec::parse("v1.2.3").unwrap(),
            UnresolvedVersionSpec::Version(Version::semantic(1, 2, 3))
        );
    }

    #[test]
    fn no_patch_becomes_req() {
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2").unwrap(),
            UnresolvedVersionSpec::Requirement(Requirement {
                major: Some(1),
                minor: Some(2),
                ..Default::default()
            })
        );
    }

    #[test]
    fn no_minor_becomes_req() {
        assert_eq!(
            UnresolvedVersionSpec::parse("1").unwrap(),
            UnresolvedVersionSpec::Requirement(Requirement {
                major: Some(1),
                ..Default::default()
            })
        );
    }

    #[test]
    fn to_partial_string() {
        assert_eq!(
            UnresolvedVersionSpec::parse("1")
                .unwrap()
                .to_partial_string(),
            "1"
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("~1.2")
                .unwrap()
                .to_partial_string(),
            "1.2"
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("^1.2.3")
                .unwrap()
                .to_partial_string(),
            "1.2.3"
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2.3-rc.0")
                .unwrap()
                .to_partial_string(),
            "1.2.3-rc.0"
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2.3+build")
                .unwrap()
                .to_partial_string(),
            "1.2.3"
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("node-1.2.3")
                .unwrap()
                .to_partial_string(),
            "node-1.2.3"
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("node-2024-02")
                .unwrap()
                .to_partial_string(),
            "node-2024.2.0"
        );
    }
}
