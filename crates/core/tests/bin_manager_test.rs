use proto_core::layout::BinManager;
use rustc_hash::FxHashMap;
use semver::Version;

mod bin_manager {
    use super::*;

    #[test]
    fn creates_buckets_per_version() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(4, 5, 6);

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
    fn highest_replaces() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 3, 4);

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
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 1, 4);

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
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 3, 4);

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
