use proto_core::VersionSpec;
use proto_core::layout::BinManager;
use rustc_hash::FxHashMap;

mod bin_manager {
    use super::*;

    #[test]
    fn creates_buckets_per_version() {
        let v1 = VersionSpec::parse("1.2.3").unwrap();
        let v2 = VersionSpec::parse("4.5.6").unwrap();

        let mut bins = BinManager::default();
        bins.add_version(&v1);
        bins.add_version(&v2);

        assert_eq!(
            bins.get_buckets(),
            FxHashMap::from_iter([
                (&"*".to_string(), &v2),
                (&"1".to_string(), &v1),
                (&"1.2".to_string(), &v1),
                (&"4".to_string(), &v2),
                (&"4.5".to_string(), &v2),
            ])
        );
    }

    #[test]
    fn creates_buckets_per_version_for_calver() {
        let v1 = VersionSpec::parse("2000-10-25").unwrap();
        let v2 = VersionSpec::parse("2100-01").unwrap();

        let mut bins = BinManager::default();
        bins.add_version(&v1);
        bins.add_version(&v2);

        assert_eq!(
            bins.get_buckets(),
            FxHashMap::from_iter([
                (&"*".to_string(), &v2),
                (&"2000".to_string(), &v1),
                (&"2000.10".to_string(), &v1),
                (&"2100".to_string(), &v2),
                (&"2100.1".to_string(), &v2),
            ])
        );
    }

    #[test]
    fn creates_bucket_for_canary() {
        let v1 = VersionSpec::Canary;

        let mut bins = BinManager::default();
        bins.add_version(&v1);

        assert_eq!(
            bins.get_buckets(),
            FxHashMap::from_iter([(&"canary".to_string(), &v1),])
        );
    }

    #[test]
    fn doesnt_create_for_aliases() {
        let v1 = VersionSpec::Alias("test".into());

        let mut bins = BinManager::default();
        bins.add_version(&v1);

        assert_eq!(bins.get_buckets(), FxHashMap::default());
    }

    #[test]
    fn highest_replaces() {
        let v1 = VersionSpec::parse("1.2.3").unwrap();
        let v2 = VersionSpec::parse("1.3.4").unwrap();

        let mut bins = BinManager::default();
        bins.add_version(&v1);

        assert_eq!(
            bins.get_buckets(),
            FxHashMap::from_iter([
                (&"*".to_string(), &v1),
                (&"1".to_string(), &v1),
                (&"1.2".to_string(), &v1),
            ])
        );

        bins.add_version(&v2);

        assert_eq!(
            bins.get_buckets(),
            FxHashMap::from_iter([
                (&"*".to_string(), &v2),
                (&"1".to_string(), &v2),
                (&"1.2".to_string(), &v1),
                (&"1.3".to_string(), &v2),
            ])
        );
    }

    #[test]
    fn lowest_doesnt_replace() {
        let v1 = VersionSpec::parse("1.2.3").unwrap();
        let v2 = VersionSpec::parse("1.1.4").unwrap();

        let mut bins = BinManager::default();
        bins.add_version(&v1);

        assert_eq!(
            bins.get_buckets(),
            FxHashMap::from_iter([
                (&"*".to_string(), &v1),
                (&"1".to_string(), &v1),
                (&"1.2".to_string(), &v1),
            ])
        );

        bins.add_version(&v2);

        assert_eq!(
            bins.get_buckets(),
            FxHashMap::from_iter([
                (&"*".to_string(), &v1),
                (&"1".to_string(), &v1),
                (&"1.1".to_string(), &v2),
                (&"1.2".to_string(), &v1),
            ])
        );
    }

    #[test]
    fn removing_rebuilds_buckets() {
        let v1 = VersionSpec::parse("1.2.3").unwrap();
        let v2 = VersionSpec::parse("1.3.4").unwrap();

        let mut bins = BinManager::default();
        bins.add_version(&v1);
        bins.add_version(&v2);

        assert_eq!(
            bins.get_buckets(),
            FxHashMap::from_iter([
                (&"*".to_string(), &v2),
                (&"1".to_string(), &v2),
                (&"1.2".to_string(), &v1),
                (&"1.3".to_string(), &v2),
            ])
        );

        bins.remove_version(&v2);

        assert_eq!(
            bins.get_buckets(),
            FxHashMap::from_iter([
                (&"*".to_string(), &v1),
                (&"1".to_string(), &v1),
                (&"1.2".to_string(), &v1),
            ])
        );
    }
}
