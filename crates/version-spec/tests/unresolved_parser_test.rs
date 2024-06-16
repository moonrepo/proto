use version_spec::{parse, ParseKind};

mod unresolved_parser {
    use super::*;

    #[test]
    fn parses_reqs() {
        assert_eq!(parse("").unwrap(), ("*".to_owned(), ParseKind::Req));
        assert_eq!(parse("*").unwrap(), ("*".to_owned(), ParseKind::Req));

        // semver
        assert_eq!(parse("1").unwrap(), ("~1".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.2").unwrap(), ("~1.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.02").unwrap(), ("~1.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("v1").unwrap(), ("~1".to_owned(), ParseKind::Req));
        assert_eq!(parse("v1.2").unwrap(), ("~1.2".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.*").unwrap(), ("~1".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.*.*").unwrap(), ("~1".to_owned(), ParseKind::Req));
        assert_eq!(parse("1.2.*").unwrap(), ("~1.2".to_owned(), ParseKind::Req));

        // calver
        assert_eq!(parse("2000").unwrap(), ("~2000".to_owned(), ParseKind::Req));
        assert_eq!(
            parse("2000-2").unwrap(),
            ("~2000.2".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse("2000-02").unwrap(),
            ("~2000.2".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse("v2000").unwrap(),
            ("~2000".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse("v2000-2").unwrap(),
            ("~2000.2".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse("2000-*").unwrap(),
            ("~2000".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse("2000-*-*").unwrap(),
            ("~2000".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse("2000-2-*").unwrap(),
            ("~2000.2".to_owned(), ParseKind::Req)
        );

        // calver (short years)
        assert_eq!(
            parse("1-2").unwrap(),
            ("~2001.2".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse("12-2").unwrap(),
            ("~2012.2".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse("123-2").unwrap(),
            ("~2123.2".to_owned(), ParseKind::Req)
        );

        for op in ["=", "<", "<=", ">", ">=", "~", "^"] {
            // semver
            assert_eq!(
                parse(format!("{op}1")).unwrap(),
                (format!("{op}1"), ParseKind::Req)
            );
            assert_eq!(
                parse(format!("{op} 1.2")).unwrap(),
                (format!("{op}1.2"), ParseKind::Req)
            );
            assert_eq!(
                parse(format!("{op}1")).unwrap(),
                (format!("{op}1"), ParseKind::Req)
            );
            assert_eq!(
                parse(format!("  {op}  v1.2.3  ")).unwrap(),
                (format!("{op}1.2.3"), ParseKind::Req)
            );

            // calver
            assert_eq!(
                parse(format!("{op}2000")).unwrap(),
                (format!("{op}2000"), ParseKind::Req)
            );
            assert_eq!(
                parse(format!("{op} 2000-10")).unwrap(),
                (format!("{op}2000.10"), ParseKind::Req)
            );
            assert_eq!(
                parse(format!("  {op}  v2000-10-03  ")).unwrap(),
                (format!("{op}2000.10.3"), ParseKind::Req)
            );
        }
    }

    #[test]
    fn parses_reqs_special() {
        assert_eq!(
            parse("1.2, 4.5").unwrap(),
            ("1.2,4.5".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse(">=1.2.7 <1.3.0").unwrap(),
            (">=1.2.7,<1.3.0".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse(">=1.2.0, <1.3.0").unwrap(),
            (">=1.2.0,<1.3.0".to_owned(), ParseKind::Req)
        );
        assert_eq!(parse("1.2.*").unwrap(), ("~1.2".to_owned(), ParseKind::Req));
        assert_eq!(
            parse(">= 1.2, < 1.5").unwrap(),
            (">=1.2,<1.5".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse(">=1.2.3 <2.4.0-0").unwrap(),
            (">=1.2.3,<2.4.0-0".to_owned(), ParseKind::Req)
        );
        assert_eq!(
            parse(">=1.2.3, <2.4.0-0").unwrap(),
            (">=1.2.3,<2.4.0-0".to_owned(), ParseKind::Req)
        );
    }

    #[test]
    fn parses_reqs_semver() {
        assert_eq!(
            parse("1.2.3").unwrap(),
            ("1.2.3".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("01.02.03").unwrap(),
            ("1.2.3".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("v1.2.3").unwrap(),
            ("1.2.3".to_owned(), ParseKind::Sem)
        );

        // pre
        assert_eq!(
            parse("1.2.3-alpha").unwrap(),
            ("1.2.3-alpha".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("1.2.3-rc.0").unwrap(),
            ("1.2.3-rc.0".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("v1.2.3-a-b-c").unwrap(),
            ("1.2.3-a-b-c".to_owned(), ParseKind::Sem)
        );

        // build
        assert_eq!(
            parse("1.2.3+alpha").unwrap(),
            ("1.2.3+alpha".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("1.2.3+rc.0").unwrap(),
            ("1.2.3+rc.0".to_owned(), ParseKind::Sem)
        );
        assert_eq!(
            parse("v1.2.3+a-b-c").unwrap(),
            ("1.2.3+a-b-c".to_owned(), ParseKind::Sem)
        );
    }

    #[test]
    fn parses_reqs_calver() {
        assert_eq!(
            parse("0-2-3").unwrap(),
            ("2000-2-3".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("00-2-3").unwrap(),
            ("2000-2-3".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("000-2-3").unwrap(),
            ("2000-2-3".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("1-2-3").unwrap(),
            ("2001-2-3".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("12-2-03").unwrap(),
            ("2012-2-3".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("123-2-31").unwrap(),
            ("2123-2-31".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("2000-2-3").unwrap(),
            ("2000-2-3".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("2000-02-03").unwrap(),
            ("2000-2-3".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("v12-2-3").unwrap(),
            ("2012-2-3".to_owned(), ParseKind::Cal)
        );

        // pre
        assert_eq!(
            parse("0-2-3-rc.0").unwrap(),
            ("2000-2-3-rc.0".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("v12-2-3-alpha-5").unwrap(),
            ("2012-2-3-alpha-5".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("12-2-3-beta").unwrap(),
            ("2012-2-3-beta".to_owned(), ParseKind::Cal)
        );

        // build
        assert_eq!(
            parse("0-2-3_123").unwrap(),
            ("2000-2-3+123".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("v12-2-3_0").unwrap(),
            ("2012-2-3+0".to_owned(), ParseKind::Cal)
        );
        assert_eq!(
            parse("12-2-3.789").unwrap(),
            ("2012-2-3+789".to_owned(), ParseKind::Cal)
        );
    }
}
