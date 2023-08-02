use proto_core2::DetectedVersion;
use semver::{Version, VersionReq};
use std::str::FromStr;

mod detected_version {
    use super::*;

    #[test]
    fn parses_alias() {
        assert_eq!(
            DetectedVersion::from_str("stable").unwrap(),
            DetectedVersion::Alias("stable".to_owned())
        );
        assert_eq!(
            DetectedVersion::from_str("latest").unwrap(),
            DetectedVersion::Alias("latest".to_owned())
        );
        assert_eq!(
            DetectedVersion::from_str("lts-2014").unwrap(),
            DetectedVersion::Alias("lts-2014".to_owned())
        );
    }

    #[test]
    fn parses_req() {
        for req in ["=1.2.3", "^1.2", "~1", ">1.2.0", "<1", "*", ">1, <=1.5"] {
            assert_eq!(
                DetectedVersion::from_str(req).unwrap(),
                DetectedVersion::ReqAll(VersionReq::parse(req).unwrap())
            );
        }
    }

    #[test]
    fn parses_req_spaces() {
        assert_eq!(
            DetectedVersion::from_str("> 10").unwrap(),
            DetectedVersion::ReqAll(VersionReq::parse(">10").unwrap())
        );
        assert_eq!(
            DetectedVersion::from_str("1.2 , 2").unwrap(),
            DetectedVersion::ReqAll(VersionReq::parse("1.2, 2").unwrap())
        );
        assert_eq!(
            DetectedVersion::from_str(">= 1.2 < 2").unwrap(),
            DetectedVersion::ReqAll(VersionReq::parse(">=1.2, <2").unwrap())
        );
    }

    #[test]
    fn parses_req_any() {
        assert_eq!(
            DetectedVersion::from_str("^1 || ~2 || =3").unwrap(),
            DetectedVersion::ReqAny(vec![
                VersionReq::parse("^1").unwrap(),
                VersionReq::parse("~2").unwrap(),
                VersionReq::parse("=3").unwrap()
            ])
        );
    }

    #[test]
    fn parses_version() {
        for req in ["1.2.3", "4.5.6", "7.8.9-alpha", "10.11.12+build"] {
            assert_eq!(
                DetectedVersion::from_str(req).unwrap(),
                DetectedVersion::Version(Version::parse(req).unwrap())
            );
        }
    }

    #[test]
    fn parses_version_with_v() {
        assert_eq!(
            DetectedVersion::from_str("v1.2.3").unwrap(),
            DetectedVersion::Version(Version::parse("1.2.3").unwrap())
        );
    }

    #[test]
    fn no_patch_becomes_req() {
        assert_eq!(
            DetectedVersion::from_str("1.2").unwrap(),
            DetectedVersion::ReqAll(VersionReq::parse("=1.2").unwrap())
        );
    }

    #[test]
    fn no_minor_becomes_req() {
        assert_eq!(
            DetectedVersion::from_str("1").unwrap(),
            DetectedVersion::ReqAll(VersionReq::parse("=1").unwrap())
        );
    }
}
