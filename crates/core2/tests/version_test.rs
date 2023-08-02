use proto_core2::VersionType;
use semver::{Version, VersionReq};
use std::str::FromStr;

mod version_type {
    use super::*;

    #[test]
    fn parses_alias() {
        assert_eq!(
            VersionType::from_str("stable").unwrap(),
            VersionType::Alias("stable".to_owned())
        );
        assert_eq!(
            VersionType::from_str("latest").unwrap(),
            VersionType::Alias("latest".to_owned())
        );
        assert_eq!(
            VersionType::from_str("lts-2014").unwrap(),
            VersionType::Alias("lts-2014".to_owned())
        );
    }

    #[test]
    fn parses_req() {
        for req in ["=1.2.3", "^1.2", "~1", ">1.2.0", "<1", "*", ">1, <=1.5"] {
            assert_eq!(
                VersionType::from_str(req).unwrap(),
                VersionType::ReqAll(VersionReq::parse(req).unwrap())
            );
        }
    }

    #[test]
    fn parses_req_spaces() {
        assert_eq!(
            VersionType::from_str("> 10").unwrap(),
            VersionType::ReqAll(VersionReq::parse(">10").unwrap())
        );
        assert_eq!(
            VersionType::from_str("1.2 , 2").unwrap(),
            VersionType::ReqAll(VersionReq::parse("1.2, 2").unwrap())
        );
        assert_eq!(
            VersionType::from_str(">= 1.2 < 2").unwrap(),
            VersionType::ReqAll(VersionReq::parse(">=1.2, <2").unwrap())
        );
    }

    #[test]
    fn parses_req_any() {
        assert_eq!(
            VersionType::from_str("^1 || ~2 || =3").unwrap(),
            VersionType::ReqAny(vec![
                VersionReq::parse("~2").unwrap(),
                VersionReq::parse("^1").unwrap(),
                VersionReq::parse("=3").unwrap(),
            ])
        );
    }

    #[test]
    fn sorts_any_req() {
        assert_eq!(
            VersionType::from_str("^1 || ^2 || ^3").unwrap(),
            VersionType::ReqAny(vec![
                VersionReq::parse("^3").unwrap(),
                VersionReq::parse("^2").unwrap(),
                VersionReq::parse("^1").unwrap(),
            ])
        );
        assert_eq!(
            VersionType::from_str("^1.1 || ^1.10 || ^1.10.1 || ^1.2").unwrap(),
            VersionType::ReqAny(vec![
                VersionReq::parse("^1.10.1").unwrap(),
                VersionReq::parse("^1.10").unwrap(),
                VersionReq::parse("^1.2").unwrap(),
                VersionReq::parse("^1.1").unwrap(),
            ])
        );
    }

    #[test]
    fn parses_version() {
        for req in ["1.2.3", "4.5.6", "7.8.9-alpha", "10.11.12+build"] {
            assert_eq!(
                VersionType::from_str(req).unwrap(),
                VersionType::Version(Version::parse(req).unwrap())
            );
        }
    }

    #[test]
    fn parses_version_with_v() {
        assert_eq!(
            VersionType::from_str("v1.2.3").unwrap(),
            VersionType::Version(Version::parse("1.2.3").unwrap())
        );
    }

    #[test]
    fn no_patch_becomes_req() {
        assert_eq!(
            VersionType::from_str("1.2").unwrap(),
            VersionType::ReqAll(VersionReq::parse("^1.2").unwrap())
        );
    }

    #[test]
    fn no_minor_becomes_req() {
        assert_eq!(
            VersionType::from_str("1").unwrap(),
            VersionType::ReqAll(VersionReq::parse("^1").unwrap())
        );
    }
}
