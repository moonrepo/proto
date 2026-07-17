// Ported from the semver crate's test suite, to verify that our behavior
// aligns with upstream wherever possible:
//
// - https://github.com/dtolnay/semver/blob/master/tests/test_version.rs
// - https://github.com/dtolnay/semver/blob/master/tests/test_version_req.rs
//
// Assertions that deliberately diverge are annotated with an "upstream:"
// comment describing the original behavior. Upstream error assertions
// include exact messages, while ours only assert that an error occurred.

use version_spec::{
    MatchesVersion, Range, Requirement, Version, parse_semver, parse_semver_range, parse_semver_req,
};

#[track_caller]
fn version(text: &str) -> Version {
    parse_semver(text).unwrap()
}

#[track_caller]
fn version_err(text: &str) {
    assert!(parse_semver(text).is_err(), "expected error for {text:?}");
}

#[track_caller]
fn req(text: &str) -> Range {
    parse_semver_range(text).unwrap()
}

#[track_caller]
fn req_err(text: &str) {
    assert!(
        parse_semver_range(text).is_err(),
        "expected error for {text:?}"
    );
}

#[track_caller]
fn comparator(text: &str) -> Requirement {
    parse_semver_req(text).unwrap()
}

#[track_caller]
fn comparator_err(text: &str) {
    assert!(
        parse_semver_req(text).is_err(),
        "expected error for {text:?}"
    );
}

#[track_caller]
fn assert_to_string(value: impl ToString, expected: &str) {
    assert_eq!(value.to_string(), expected);
}

#[track_caller]
fn assert_match_all(range: &Range, versions: &[&str]) {
    for string in versions {
        assert!(range.matches(&version(string)), "did not match {string}");
    }
}

#[track_caller]
fn assert_match_none(range: &Range, versions: &[&str]) {
    for string in versions {
        assert!(!range.matches(&version(string)), "matched {string}");
    }
}

mod version {
    use super::*;

    #[test]
    fn test_parse() {
        version_err("");
        version_err("  ");
        version_err("1");
        version_err("1.2");
        version_err("1.2.3-");
        version_err("a.b.c");
        version_err("1.2.3 abc");
        version_err("1.2.3++");
        version_err("07");
        version_err("111111111111111111111.0.0");
        version_err("8\0");

        // upstream: rejects leading zeros in numeric pre-release
        // identifiers, while our grammar allows them
        version("1.2.3-01");

        let parsed = version("1.2.3");
        assert_eq!(parsed, Version::semantic(1, 2, 3));
        assert_eq!(
            parsed,
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                ..Default::default()
            }
        );

        assert_eq!(
            version("1.2.3-alpha1"),
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                prerelease: Some("alpha1".into()),
                ..Default::default()
            }
        );

        assert_eq!(
            version("1.2.3+build5"),
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                build: Some("build5".into()),
                ..Default::default()
            }
        );

        assert_eq!(
            version("1.2.3+5build"),
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                build: Some("5build".into()),
                ..Default::default()
            }
        );

        assert_eq!(
            version("1.2.3-alpha1+build5"),
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                prerelease: Some("alpha1".into()),
                build: Some("build5".into()),
                ..Default::default()
            }
        );

        assert_eq!(
            version("1.2.3-1.alpha1.9+build5.7.3aedf"),
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                prerelease: Some("1.alpha1.9".into()),
                build: Some("build5.7.3aedf".into()),
                ..Default::default()
            }
        );

        assert_eq!(
            version("1.2.3-0a.alpha1.9+05build.7.3aedf"),
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                prerelease: Some("0a.alpha1.9".into()),
                build: Some("05build.7.3aedf".into()),
                ..Default::default()
            }
        );

        assert_eq!(
            version("0.4.0-beta.1+0851523"),
            Version {
                major: 0,
                minor: 4,
                patch: 0,
                prerelease: Some("beta.1".into()),
                build: Some("0851523".into()),
                ..Default::default()
            }
        );

        // for https://nodejs.org/dist/index.json, where some older npm versions are "1.1.0-beta-10"
        assert_eq!(
            version("1.1.0-beta-10"),
            Version {
                major: 1,
                minor: 1,
                patch: 0,
                prerelease: Some("beta-10".into()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_eq() {
        assert_eq!(version("1.2.3"), version("1.2.3"));
        assert_eq!(version("1.2.3-alpha1"), version("1.2.3-alpha1"));
        assert_eq!(version("1.2.3+build.42"), version("1.2.3+build.42"));
        assert_eq!(version("1.2.3-alpha1+42"), version("1.2.3-alpha1+42"));
    }

    #[test]
    fn test_ne() {
        assert_ne!(version("0.0.0"), version("0.0.1"));
        assert_ne!(version("0.0.0"), version("0.1.0"));
        assert_ne!(version("0.0.0"), version("1.0.0"));
        assert_ne!(version("1.2.3-alpha"), version("1.2.3-beta"));
        assert_ne!(version("1.2.3+23"), version("1.2.3+42"));
    }

    #[test]
    fn test_display() {
        assert_to_string(version("1.2.3"), "1.2.3");
        assert_to_string(version("1.2.3-alpha1"), "1.2.3-alpha1");
        assert_to_string(version("1.2.3+build.42"), "1.2.3+build.42");
        assert_to_string(version("1.2.3-alpha1+42"), "1.2.3-alpha1+42");
    }

    #[test]
    fn test_lt() {
        assert!(version("0.0.0") < version("1.2.3-alpha2"));
        assert!(version("1.0.0") < version("1.2.3-alpha2"));
        assert!(version("1.2.0") < version("1.2.3-alpha2"));
        assert!(version("1.2.3-alpha1") < version("1.2.3"));
        assert!(version("1.2.3-alpha1") < version("1.2.3-alpha2"));
        assert!(!(version("1.2.3-alpha2") < version("1.2.3-alpha2")));
        assert!(version("1.2.3+23") < version("1.2.3+42"));
    }

    #[test]
    fn test_le() {
        assert!(version("0.0.0") <= version("1.2.3-alpha2"));
        assert!(version("1.0.0") <= version("1.2.3-alpha2"));
        assert!(version("1.2.0") <= version("1.2.3-alpha2"));
        assert!(version("1.2.3-alpha1") <= version("1.2.3-alpha2"));
        assert!(version("1.2.3-alpha2") <= version("1.2.3-alpha2"));
        assert!(version("1.2.3+23") <= version("1.2.3+42"));
    }

    #[test]
    fn test_gt() {
        assert!(version("1.2.3-alpha2") > version("0.0.0"));
        assert!(version("1.2.3-alpha2") > version("1.0.0"));
        assert!(version("1.2.3-alpha2") > version("1.2.0"));
        assert!(version("1.2.3-alpha2") > version("1.2.3-alpha1"));
        assert!(version("1.2.3") > version("1.2.3-alpha2"));
        assert!(!(version("1.2.3-alpha2") > version("1.2.3-alpha2")));
        assert!(!(version("1.2.3+23") > version("1.2.3+42")));
    }

    #[test]
    fn test_ge() {
        assert!(version("1.2.3-alpha2") >= version("0.0.0"));
        assert!(version("1.2.3-alpha2") >= version("1.0.0"));
        assert!(version("1.2.3-alpha2") >= version("1.2.0"));
        assert!(version("1.2.3-alpha2") >= version("1.2.3-alpha1"));
        assert!(version("1.2.3-alpha2") >= version("1.2.3-alpha2"));
        assert!(!(version("1.2.3+23") >= version("1.2.3+42")));
    }

    #[test]
    fn test_spec_order() {
        let vs = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        let mut i = 1;
        while i < vs.len() {
            let a = version(vs[i - 1]);
            let b = version(vs[i]);
            assert!(a < b, "nope {a:?} < {b:?}");
            i += 1;
        }
    }
}

mod version_req {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    #[test]
    fn test_basic() {
        let r = &req("1.0.0");
        // upstream: a bare requirement defaults to caret ("^1.0.0"), while
        // ours defaults to tilde, so "1.1.0" is excluded rather than matched
        assert_to_string(r, "~1.0.0");
        assert_match_all(r, &["1.0.0", "1.0.1"]);
        assert_match_none(
            r,
            &[
                "0.9.9",
                "0.10.0",
                "0.1.0",
                "1.1.0",
                "1.0.0-pre",
                "1.0.1-pre",
            ],
        );
    }

    #[test]
    fn test_default() {
        // upstream: VersionReq::default() equals VersionReq::STAR
        assert_eq!(Range::default(), req("*"));
    }

    #[test]
    fn test_exact() {
        let r = &req("=1.0.0");
        assert_to_string(r, "=1.0.0");
        assert_match_all(r, &["1.0.0"]);
        assert_match_none(r, &["1.0.1", "0.9.9", "0.10.0", "0.1.0", "1.0.0-pre"]);

        let r = &req("=0.9.0");
        assert_to_string(r, "=0.9.0");
        assert_match_all(r, &["0.9.0"]);
        assert_match_none(r, &["0.9.1", "1.9.0", "0.0.9", "0.9.0-pre"]);

        let r = &req("=0.0.2");
        assert_to_string(r, "=0.0.2");
        assert_match_all(r, &["0.0.2"]);
        assert_match_none(r, &["0.0.1", "0.0.3", "0.0.2-pre"]);

        let r = &req("=0.1.0-beta2.a");
        assert_to_string(r, "=0.1.0-beta2.a");
        assert_match_all(r, &["0.1.0-beta2.a"]);
        assert_match_none(r, &["0.9.1", "0.1.0", "0.1.1-beta2.a", "0.1.0-beta2"]);

        let r = &req("=0.1.0+meta");
        assert_to_string(r, "=0.1.0");
        assert_match_all(r, &["0.1.0", "0.1.0+meta", "0.1.0+any"]);
    }

    #[test]
    fn test_greater_than() {
        let r = &req(">= 1.0.0");
        assert_to_string(r, ">=1.0.0");
        assert_match_all(r, &["1.0.0", "2.0.0"]);
        assert_match_none(r, &["0.1.0", "0.0.1", "1.0.0-pre", "2.0.0-pre"]);

        let r = &req(">= 2.1.0-alpha2");
        assert_to_string(r, ">=2.1.0-alpha2");
        assert_match_all(r, &["2.1.0-alpha2", "2.1.0-alpha3", "2.1.0", "3.0.0"]);
        assert_match_none(
            r,
            &["2.0.0", "2.1.0-alpha1", "2.0.0-alpha2", "3.0.0-alpha2"],
        );
    }

    #[test]
    fn test_less_than() {
        let r = &req("< 1.0.0");
        assert_to_string(r, "<1.0.0");
        assert_match_all(r, &["0.1.0", "0.0.1"]);
        assert_match_none(r, &["1.0.0", "1.0.0-beta", "1.0.1", "0.9.9-alpha"]);

        let r = &req("<= 2.1.0-alpha2");
        assert_match_all(r, &["2.1.0-alpha2", "2.1.0-alpha1", "2.0.0", "1.0.0"]);
        assert_match_none(
            r,
            &["2.1.0", "2.2.0-alpha1", "2.0.0-alpha2", "1.0.0-alpha2"],
        );

        let r = &req(">1.0.0-alpha, <1.0.0");
        assert_match_all(r, &["1.0.0-beta"]);

        let r = &req(">1.0.0-alpha, <1.0");
        assert_match_none(r, &["1.0.0-beta"]);

        let r = &req(">1.0.0-alpha, <1");
        assert_match_none(r, &["1.0.0-beta"]);
    }

    #[test]
    fn test_multiple() {
        let r = &req("> 0.0.9, <= 2.5.3");
        // upstream: rendered with a comma separator, ">0.0.9, <=2.5.3"
        assert_to_string(r, ">0.0.9 && <=2.5.3");
        assert_match_all(r, &["0.0.10", "1.0.0", "2.5.3"]);
        assert_match_none(r, &["0.0.8", "2.5.4"]);

        let r = &req("0.3.0, 0.4.0");
        // upstream: bare requirements default to caret ("^0.3.0 && ^0.4.0"),
        // though the matched set is identical for these `0.x` versions
        assert_to_string(r, "~0.3.0 && ~0.4.0");
        assert_match_none(r, &["0.0.8", "0.3.0", "0.4.0"]);

        let r = &req("<= 0.2.0, >= 0.5.0");
        assert_to_string(r, "<=0.2.0 && >=0.5.0");
        assert_match_none(r, &["0.0.8", "0.3.0", "0.5.1"]);

        let r = &req("0.1.0, 0.1.4, 0.1.6");
        // upstream: rendered with comma separators and a caret default
        assert_to_string(r, "~0.1.0 && ~0.1.4 && ~0.1.6");
        assert_match_all(r, &["0.1.6", "0.1.9"]);
        assert_match_none(r, &["0.1.0", "0.1.4", "0.2.0"]);

        req_err("> 0.1.0,");
        req_err("> 0.3.0, ,");

        let r = &req(">=0.5.1-alpha3, <0.6");
        assert_to_string(r, ">=0.5.1-alpha3 && <0.6");
        assert_match_all(
            r,
            &[
                "0.5.1-alpha3",
                "0.5.1-alpha4",
                "0.5.1-beta",
                "0.5.1",
                "0.5.5",
            ],
        );
        assert_match_none(
            r,
            &["0.5.1-alpha1", "0.5.2-alpha3", "0.5.5-pre", "0.5.0-pre"],
        );
        assert_match_none(r, &["0.6.0", "0.6.0-pre"]);

        // upstream: hyphen ranges are not supported and error,
        // while ours parses them as a bounded range
        let r = &req("1.2.3 - 2.3.4");
        assert_match_all(r, &["1.2.3", "2.0.0", "2.3.4"]);
        assert_match_none(r, &["1.2.2", "2.3.5"]);

        // upstream: errors with an excessive number of comparators,
        // while ours is unbounded
        req(
            ">1, >2, >3, >4, >5, >6, >7, >8, >9, >10, >11, >12, >13, >14, >15, >16, >17, >18, >19, >20, >21, >22, >23, >24, >25, >26, >27, >28, >29, >30, >31, >32, >33",
        );
    }

    #[test]
    fn test_whitespace_delimited_comparator_sets() {
        // upstream: space separated comparators error,
        // while ours treats a space as an "and"
        let r = &req("> 0.0.9 <= 2.5.3");
        assert_match_all(r, &["0.0.10", "1.0.0", "2.5.3"]);
        assert_match_none(r, &["0.0.8", "2.5.4"]);
    }

    #[test]
    fn test_tilde() {
        let r = &req("~1");
        assert_match_all(r, &["1.0.0", "1.0.1", "1.1.1"]);
        assert_match_none(r, &["0.9.1", "2.9.0", "0.0.9"]);

        let r = &req("~1.2");
        assert_match_all(r, &["1.2.0", "1.2.1"]);
        assert_match_none(r, &["1.1.1", "1.3.0", "0.0.9"]);

        let r = &req("~1.2.2");
        assert_match_all(r, &["1.2.2", "1.2.4"]);
        assert_match_none(r, &["1.2.1", "1.9.0", "1.0.9", "2.0.1", "0.1.3"]);

        let r = &req("~1.2.3-beta.2");
        assert_match_all(r, &["1.2.3", "1.2.4", "1.2.3-beta.2", "1.2.3-beta.4"]);
        assert_match_none(r, &["1.3.3", "1.1.4", "1.2.3-beta.1", "1.2.4-beta.2"]);
    }

    #[test]
    fn test_caret() {
        let r = &req("^1");
        assert_match_all(r, &["1.1.2", "1.1.0", "1.2.1", "1.0.1"]);
        assert_match_none(r, &["0.9.1", "2.9.0", "0.1.4"]);
        assert_match_none(r, &["1.0.0-beta1", "0.1.0-alpha", "1.0.1-pre"]);

        let r = &req("^1.1");
        assert_match_all(r, &["1.1.2", "1.1.0", "1.2.1"]);
        assert_match_none(r, &["0.9.1", "2.9.0", "1.0.1", "0.1.4"]);

        let r = &req("^1.1.2");
        assert_match_all(r, &["1.1.2", "1.1.4", "1.2.1"]);
        assert_match_none(r, &["0.9.1", "2.9.0", "1.1.1", "0.0.1"]);
        assert_match_none(r, &["1.1.2-alpha1", "1.1.3-alpha1", "2.9.0-alpha1"]);

        let r = &req("^0.1.2");
        assert_match_all(r, &["0.1.2", "0.1.4"]);
        assert_match_none(r, &["0.9.1", "2.9.0", "1.1.1", "0.0.1"]);
        assert_match_none(r, &["0.1.2-beta", "0.1.3-alpha", "0.2.0-pre"]);

        let r = &req("^0.5.1-alpha3");
        assert_match_all(
            r,
            &[
                "0.5.1-alpha3",
                "0.5.1-alpha4",
                "0.5.1-beta",
                "0.5.1",
                "0.5.5",
            ],
        );
        assert_match_none(
            r,
            &[
                "0.5.1-alpha1",
                "0.5.2-alpha3",
                "0.5.5-pre",
                "0.5.0-pre",
                "0.6.0",
            ],
        );

        let r = &req("^0.0.2");
        assert_match_all(r, &["0.0.2"]);
        assert_match_none(r, &["0.9.1", "2.9.0", "1.1.1", "0.0.1", "0.1.4"]);

        let r = &req("^0.0");
        assert_match_all(r, &["0.0.2", "0.0.0"]);
        assert_match_none(r, &["0.9.1", "2.9.0", "1.1.1", "0.1.4"]);

        let r = &req("^0");
        assert_match_all(r, &["0.9.1", "0.0.2", "0.0.0"]);
        assert_match_none(r, &["2.9.0", "1.1.1"]);

        let r = &req("^1.4.2-beta.5");
        assert_match_all(
            r,
            &["1.4.2", "1.4.3", "1.4.2-beta.5", "1.4.2-beta.6", "1.4.2-c"],
        );
        assert_match_none(
            r,
            &[
                "0.9.9",
                "2.0.0",
                "1.4.2-alpha",
                "1.4.2-beta.4",
                "1.4.3-beta.5",
            ],
        );
    }

    #[test]
    fn test_wildcard() {
        // upstream: an empty requirement errors,
        // while ours is a wildcard match
        assert_eq!(req(""), req("*"));

        let r = &req("*");
        assert_match_all(r, &["0.9.1", "2.9.0", "0.0.9", "1.0.1", "1.1.1"]);
        assert_match_none(r, &["1.0.0-pre"]);

        for s in ["x", "X"] {
            assert_eq!(*r, req(s));
        }

        let r = &req("1.*");
        assert_match_all(r, &["1.2.0", "1.2.1", "1.1.1", "1.3.0"]);
        assert_match_none(r, &["0.0.9", "1.2.0-pre"]);

        for s in ["1.x", "1.X", "1.*.*"] {
            assert_eq!(*r, req(s));
        }

        let r = &req("1.2.*");
        assert_match_all(r, &["1.2.0", "1.2.2", "1.2.4"]);
        assert_match_none(r, &["1.9.0", "1.0.9", "2.0.1", "0.1.3", "1.2.2-pre"]);

        for s in ["1.2.x", "1.2.X"] {
            assert_eq!(*r, req(s));
        }
    }

    #[test]
    fn test_logical_or() {
        // upstream: logical OR is not supported and errors,
        // while ours parses alternate clauses
        let r = &req("=1.2.3 || =2.3.4");
        assert_match_all(r, &["1.2.3", "2.3.4"]);
        assert_match_none(r, &["1.2.4", "2.3.3"]);

        let r = &req("1.1 || =1.2.3");
        assert_match_all(r, &["1.1.0", "1.1.9", "1.2.3"]);

        let r = &req("6.* || 8.* || >= 10.*");
        assert_match_all(r, &["6.1.0", "8.2.3", "10.0.0", "12.0.0"]);
        assert_match_none(r, &["7.0.0", "9.0.0"]);
    }

    #[test]
    fn test_any() {
        let r = &Range::default();
        assert_match_all(r, &["0.0.1", "0.1.0", "1.0.0"]);
    }

    #[test]
    fn test_pre() {
        let r = &req("=2.1.1-really.0");
        assert_match_all(r, &["2.1.1-really.0"]);
    }

    #[test]
    fn test_parse() {
        req_err("\0");
        req_err(">= >= 0.0.2");
        req_err(">== 0.0.2");
        req_err("a.0.0");
        req_err("1.0.0-");
        req_err(">=");
    }

    #[test]
    fn test_comparator_parse() {
        let parsed = comparator("1.2.3-alpha");
        // upstream: a bare requirement defaults to caret ("^1.2.3-alpha")
        assert_to_string(parsed, "~1.2.3-alpha");

        let parsed = comparator("2.X");
        assert_to_string(parsed, "2.*");

        let parsed = comparator("2");
        // upstream: a bare requirement defaults to caret ("^2")
        assert_to_string(parsed, "~2");

        let parsed = comparator("2.x.x");
        assert_to_string(parsed, "2.*");

        // upstream: rejects leading zeros in numeric pre-release
        // identifiers, while our grammar allows them
        comparator("1.2.3-01");

        // upstream: errors on the empty identifier segment, while our
        // grammar is loose, and the metadata is ignored anyway
        comparator("1.2.3+4.");

        comparator_err(">");
        comparator_err("1.");
        comparator_err("1.*.");
        comparator_err("1.2.3+4ÿ");
    }

    #[test]
    fn test_cargo3202() {
        let r = &req("0.*.*");
        assert_to_string(r, "0.*");
        assert_match_all(r, &["0.5.0"]);

        let r = &req("0.0.*");
        assert_to_string(r, "0.0.*");
    }

    #[test]
    fn test_digit_after_wildcard() {
        req_err("*.1");
        req_err("1.*.1");
        req_err(">=1.*.1");
    }

    #[test]
    fn test_eq_hash() {
        fn calculate_hash(value: impl Hash) -> u64 {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        assert_eq!(req("^1"), req("^1"));
        assert_eq!(calculate_hash(req("^1")), calculate_hash(req("^1")));
        assert_ne!(req("^1"), req("^2"));
    }

    #[test]
    fn test_leading_digit_in_pre_and_build() {
        for op in ["=", ">", ">=", "<", "<=", "~", "^"] {
            // digit then alpha
            req(&format!("{op} 1.2.3-1a"));
            req(&format!("{op} 1.2.3+1a"));

            // digit then alpha (leading zero)
            req(&format!("{op} 1.2.3-01a"));
            req(&format!("{op} 1.2.3+01"));

            // multiple
            req(&format!("{op} 1.2.3-1+1"));
            req(&format!("{op} 1.2.3-1-1+1-1-1"));
            req(&format!("{op} 1.2.3-1a+1a"));
            req(&format!("{op} 1.2.3-1a-1a+1a-1a-1a"));
        }
    }

    #[test]
    fn test_wildcard_and_another() {
        // upstream: a wildcard must be the only comparator, while
        // ours allows a wildcard requirement within a clause
        req("*, 0.20.0-any");
        req("0.20.0-any, *");

        req("0.20.0-any, *, 1.0");
    }
}
