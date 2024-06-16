use version_spec::{parse, ParseKind};

mod unresolved_parser {
    use super::*;

    #[test]
    fn parses_reqs() {
        assert_eq!(parse(""), ("*".to_owned(), ParseKind::Req));
        assert_eq!(parse("*"), ("*".to_owned(), ParseKind::Req));

        // semver
        assert_eq!(parse("1"), ("~1".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.2"), ("~1.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.02"), ("~1.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("v1"), ("~1".to_owned(), ParseKind::Req));
        assert_eq!(parse("v1.2"), ("~1.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.*"), ("~1".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.*.*"), ("~1".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.2.*"), ("~1.2".to_owned(), ParseKind::Req));

        // calver
        assert_eq!(parse("2000"), ("~2000".to_owned(), ParseKind::Req));
        assert_eq!(parse("2000-2"), ("~2000.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("2000-02"), ("~2000.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("v2000"), ("~2000".to_owned(), ParseKind::Req));
        assert_eq!(parse("v2000-2"), ("~2000.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("2000-*"), ("~2000".to_owned(), ParseKind::Req));
        assert_eq!(parse("2000-*-*"), ("~2000".to_owned(), ParseKind::Req));
        assert_eq!(parse("2000-2-*"), ("~2000.2".to_owned(), ParseKind::Req));

        // calver (short years)
        assert_eq!(parse("1-2"), ("~2001.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("12-2"), ("~2012.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("123-2"), ("~2123.2".to_owned(), ParseKind::Req));

        for op in ["=", "<", "<=", ">", ">=", "~", "^"] {
            // semver
            assert_eq!(parse(format!("{op}1")), (format!("{op}1"), ParseKind::Req));
            assert_eq!(
                parse(format!("{op} 1.2")),
                (format!("{op}1.2"), ParseKind::Req)
            );
            assert_eq!(parse(format!("{op}1")), (format!("{op}1"), ParseKind::Req));
            assert_eq!(
                parse(format!("  {op}  v1.2.3  ")),
                (format!("{op}1.2.3"), ParseKind::Req)
            );

            // calver
            assert_eq!(
                parse(format!("{op}2000")),
                (format!("{op}2000"), ParseKind::Req)
            );
            assert_eq!(
                parse(format!("{op} 2000-10")),
                (format!("{op}2000.10"), ParseKind::Req)
            );
            assert_eq!(
                parse(format!("  {op}  v2000-10-03  ")),
                (format!("{op}2000.10.3"), ParseKind::Req)
            );
        }
    }

    #[test]
    fn parses_reqs_special() {
        assert_eq!(parse("1.2, 4.5"), ("1.2,4.5".to_owned(), ParseKind::Req));
        assert_eq!(
            parse(">=1.2.7 <1.3.0"),
            (">=1.2.7,<1.3.0".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse(">=1.2.0, <1.3.0"),
            (">=1.2.0,<1.3.0".to_owned(), ParseKind::Req)
        );
        assert_eq!(parse("1.2.*"), ("~1.2".to_owned(), ParseKind::Req));
        assert_eq!(
            parse(">= 1.2, < 1.5"),
            (">=1.2,<1.5".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse(">=1.2.3 <2.4.0-0"),
            (">=1.2.3,<2.4.0-0".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse(">=1.2.3, <2.4.0-0"),
            (">=1.2.3,<2.4.0-0".to_owned(), ParseKind::Req)
        );
    }

    #[test]
    fn parses_reqs_semver() {
        assert_eq!(parse("1.2.3"), ("1.2.3".to_owned(), ParseKind::Sem));
        assert_eq!(parse("01.02.03"), ("1.2.3".to_owned(), ParseKind::Sem));
        assert_eq!(parse("v1.2.3"), ("1.2.3".to_owned(), ParseKind::Sem));

        // pre
        assert_eq!(
            parse("1.2.3-alpha"),
            ("1.2.3-alpha".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("1.2.3-rc.0"),
            ("1.2.3-rc.0".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("v1.2.3-a-b-c"),
            ("1.2.3-a-b-c".to_owned(), ParseKind::Sem)
        );

        // build
        assert_eq!(
            parse("1.2.3+alpha"),
            ("1.2.3+alpha".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("1.2.3+rc.0"),
            ("1.2.3+rc.0".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("v1.2.3+a-b-c"),
            ("1.2.3+a-b-c".to_owned(), ParseKind::Sem)
        );
    }

    #[test]
    fn parses_reqs_calver() {
        assert_eq!(parse("0-2-3"), ("2000-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("00-2-3"), ("2000-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("000-2-3"), ("2000-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("1-2-3"), ("2001-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("12-2-03"), ("2012-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("123-2-31"), ("2123-2-31".to_owned(), ParseKind::Cal));
        assert_eq!(parse("2000-2-3"), ("2000-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("2000-02-03"), ("2000-2-3".to_owned(), ParseKind::Cal));
        assert_eq!(parse("v12-2-3"), ("2012-2-3".to_owned(), ParseKind::Cal));

        // pre
        assert_eq!(
            parse("0-2-3-rc.0"),
            ("2000-2-3-rc.0".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("v12-2-3-alpha-5"),
            ("2012-2-3-alpha-5".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("12-2-3-beta"),
            ("2012-2-3-beta".to_owned(), ParseKind::Cal)
        );

        // build
        assert_eq!(
            parse("0-2-3_123"),
            ("2000-2-3+123".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("v12-2-3_0"),
            ("2012-2-3+0".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("12-2-3.789"),
            ("2012-2-3+789".to_owned(), ParseKind::Cal)
        );
    }
}
