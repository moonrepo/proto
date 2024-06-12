use semver::{Version, VersionReq};
use version_spec::{SemVer, UnresolvedVersionSpec};

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
            UnresolvedVersionSpec::Alias("latest".to_owned())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("stable").unwrap(),
            UnresolvedVersionSpec::Alias("stable".to_owned())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("legacy-2023").unwrap(),
            UnresolvedVersionSpec::Alias("legacy-2023".to_owned())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("future/202x").unwrap(),
            UnresolvedVersionSpec::Alias("future/202x".to_owned())
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
    }

    #[test]
    fn requirements() {
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1.2").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.2.*").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1.2").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1.*").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("~1").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse(">1").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse(">1").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("<=1").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("<=1").unwrap())
        );
        assert_eq!(
            UnresolvedVersionSpec::parse("1, 2").unwrap(),
            UnresolvedVersionSpec::Req(VersionReq::parse("1, 2").unwrap())
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
    }
}
