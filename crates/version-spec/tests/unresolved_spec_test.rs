use compact_str::CompactString;
use semver::{Version, VersionReq};
use version_spec::{CalVer, SemVer, UnresolvedVersionSpec};

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
        assert_eq!(
            UnresolvedVersionSpec::parse("legacy-2023").unwrap(),
            UnresolvedVersionSpec::Alias(CompactString::new("legacy-2023"))
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
            UnresolvedVersionSpec::Semantic(SemVer(Version::new(1, 2, 3)))
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2.3").unwrap(),
            UnresolvedVersionSpec::Semantic(SemVer(Version::new(1, 2, 3)))
        );

        // calver
        assert_eq!(
            UnresolvedVersionSpec::parse("2024-02").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~2024.2").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("2024-2-26").unwrap(),
            UnresolvedVersionSpec::Calendar(CalVer(Version::new(2024, 2, 26)))
        );
    }

    #[test]
    fn requirements() {
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1.2").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("~2000-2").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~2000.2").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("2000").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~2000").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2.*").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1.2").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("2000.02.*").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~2000.2").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.*").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("2000.*").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~2000").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.x").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("2000.x").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~2000").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.x.x").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("2000.x.x").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~2000").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2.X").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1.2").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("2000.02.X").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~2000.2").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse(">1").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse(">1").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse(">2000-10").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse(">2000.10").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("<=1").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("<=1").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("<=2000-12-12").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("<=2000.12.12").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1, 2").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("1, 2").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("2000-05, 3000-01").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("2000.5, 3000.1").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1,2").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("1,2").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1 2").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("1, 2").unwrap())
        );
    }

    #[test]
    fn any_requirements() {
        assert_eq!(
            UnresolvedVersionSpec::parse("^1.2 || ~1 || 3,4").unwrap(),
            UnresolvedVersionSpec::ReqAny(vec![
                VersionReq::parse("~1").unwrap(),
                VersionReq::parse("^1.2").unwrap(),
                VersionReq::parse("3,4").unwrap(),
            ])
        );

        assert_eq!(
            UnresolvedVersionSpec::parse("^2000-10 || ~1000 || 3000-05-12,4000-09-09").unwrap(),
            UnresolvedVersionSpec::ReqAny(vec![
                VersionReq::parse("~1000").unwrap(),
                VersionReq::parse("^2000.10").unwrap(),
                VersionReq::parse("3000.5.12,4000.9.9").unwrap(),
            ])
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
            UnresolvedVersionSpec::parse("lts-2014").unwrap(),
            UnresolvedVersionSpec::Alias(CompactString::new("lts-2014"))
        );
    }

    #[test]
    fn parses_req() {
        for req in ["=1.2.3", "^1.2", "~1", ">1.2.0", "<1", "*", ">1, <=1.5"] {
            assert_eq!(
                UnresolvedVersionSpec::parse(req).unwrap(),
                UnresolvedVersionSpec::Req(VersionReq::parse(req).unwrap())
            );
        }
    }

    #[test]
    fn parses_req_spaces() {
        assert_eq!(
            UnresolvedVersionSpec::parse("> 10").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse(">10").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2 , 2").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("1.2, 2").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse(">= 1.2 < 2").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse(">=1.2, <2").unwrap())
        );
    }

    #[test]
    fn parses_req_any() {
        assert_eq!(
            UnresolvedVersionSpec::parse("^1 || ~2 || =3").unwrap(),
            UnresolvedVersionSpec::ReqAny(vec![
                VersionReq::parse("~2").unwrap(),
                VersionReq::parse("^1").unwrap(),
                VersionReq::parse("=3").unwrap(),
            ])
        );
    }

    #[test]
    fn sorts_any_req() {
        assert_eq!(
            UnresolvedVersionSpec::parse("^1 || ^2 || ^3").unwrap(),
            UnresolvedVersionSpec::ReqAny(vec![
                VersionReq::parse("^3").unwrap(),
                VersionReq::parse("^2").unwrap(),
                VersionReq::parse("^1").unwrap(),
            ])
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("^1.1 || ^1.10 || ^1.10.1 || ^1.2").unwrap(),
            UnresolvedVersionSpec::ReqAny(vec![
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
                UnresolvedVersionSpec::parse(req).unwrap(),
                UnresolvedVersionSpec::Semantic(SemVer(Version::parse(req).unwrap()))
            );
        }
    }

    #[test]
    fn parses_version_with_v() {
        assert_eq!(
            UnresolvedVersionSpec::parse("v1.2.3").unwrap(),
            UnresolvedVersionSpec::Semantic(SemVer(Version::parse("1.2.3").unwrap()))
        );
    }

    #[test]
    fn no_patch_becomes_req() {
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1.2").unwrap())
        );
    }

    #[test]
    fn no_minor_becomes_req() {
        assert_eq!(
            UnresolvedVersionSpec::parse("1").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1").unwrap())
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
    }
}
