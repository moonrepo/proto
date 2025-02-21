use semver::{BuildMetadata, Prerelease, Version};
use version_spec::{CalVer, is_calver};

mod calver {
    use super::*;

    #[test]
    fn matches() {
        let years = [
            "2024", // 4 digit
            "224",  // 3 digit
            "24",   // 2 digit
            "4",    // 1 digit
            "04",   // zero padded
        ];
        let months = [
            "3",  // without zero
            "03", // zero padded
            "12", // 2 digit
        ];
        // let weeks = ["1", "25", "52", "05"];
        let days = ["1", "18", "30", "09"];

        for year in years {
            for month in months {
                assert!(is_calver(format!("{year}-{month}")));
                assert!(is_calver(format!("{year}-{month}-rc.1")));
                assert!(is_calver(format!("{year}-{month}_456")));
                assert!(is_calver(format!("{year}-{month}_456-rc.2")));
                assert!(is_calver(format!("{year}-{month}.456")));

                for day in days {
                    assert!(is_calver(format!("{year}-{month}-{day}")));
                    assert!(is_calver(format!("{year}-{month}-{day}-beta.1")));
                    assert!(is_calver(format!("{year}-{month}-{day}_123")));
                    assert!(is_calver(format!("{year}-{month}-{day}.123")));
                    assert!(is_calver(format!("{year}-{month}-{day}.123-beta.1")));
                }
            }
        }
    }

    #[test]
    fn doesnt_match() {
        // invalid
        assert!(!is_calver("24"));
        assert!(!is_calver("2024"));

        // invalid months
        assert!(!is_calver("2024-0"));
        assert!(!is_calver("2024-00"));
        assert!(!is_calver("2024-13"));
        assert!(!is_calver("2024-20"));
        assert!(!is_calver("2024-010"));

        // invalid days
        assert!(!is_calver("2024-10-0"));
        assert!(!is_calver("2024-10-00"));
        assert!(!is_calver("2024-10-123"));
        assert!(!is_calver("2024-10-023"));
        assert!(!is_calver("2024-10-40"));
        assert!(!is_calver("2024-10-50"));

        // invalid micro
        assert!(!is_calver("2024_abc"));
        assert!(!is_calver("2024-10_abc"));
        assert!(!is_calver("2024-1-1_abc"));
    }

    #[test]
    fn parse_year_month() {
        for (month, actual) in [("1", "01"), ("05", "05"), ("10", "10"), ("12", "12")] {
            let ver = CalVer::parse(&format!("2024-{month}")).unwrap();

            assert_eq!(
                ver.0,
                Version {
                    major: 2024,
                    minor: actual.parse().unwrap(),
                    patch: 0,
                    pre: Prerelease::EMPTY,
                    build: BuildMetadata::EMPTY,
                }
            );
            assert_eq!(ver.to_string(), format!("2024-{actual}"));
        }

        // build
        let ver = CalVer::parse("2024-5_123").unwrap();

        assert_eq!(
            ver.0,
            Version {
                major: 2024,
                minor: 5,
                patch: 0,
                pre: Prerelease::EMPTY,
                build: BuildMetadata::new("123").unwrap(),
            }
        );
        assert_eq!(ver.to_string(), "2024-05.123");

        // pre
        let ver = CalVer::parse("2024-05-alpha.1").unwrap();

        assert_eq!(
            ver.0,
            Version {
                major: 2024,
                minor: 5,
                patch: 0,
                pre: Prerelease::new("alpha.1").unwrap(),
                build: BuildMetadata::EMPTY,
            }
        );
        assert_eq!(ver.to_string(), "2024-05-alpha.1");

        // pre + build
        let ver = CalVer::parse("2024-05_123-alpha.1").unwrap();

        assert_eq!(
            ver.0,
            Version {
                major: 2024,
                minor: 5,
                patch: 0,
                pre: Prerelease::new("alpha.1").unwrap(),
                build: BuildMetadata::new("123").unwrap(),
            }
        );
        assert_eq!(ver.to_string(), "2024-05.123-alpha.1");
    }

    #[test]
    fn parse_year_month_day() {
        for (day, actual) in [
            ("1", "01"),
            ("05", "05"),
            ("10", "10"),
            ("22", "22"),
            ("31", "31"),
        ] {
            let ver = CalVer::parse(&format!("2024-1-{day}")).unwrap();

            assert_eq!(
                ver.0,
                Version {
                    major: 2024,
                    minor: 1,
                    patch: actual.parse().unwrap(),
                    pre: Prerelease::EMPTY,
                    build: BuildMetadata::EMPTY,
                }
            );
            assert_eq!(ver.to_string(), format!("2024-01-{actual}"));
        }

        // build
        let ver = CalVer::parse("2024-5-23_123").unwrap();

        assert_eq!(
            ver.0,
            Version {
                major: 2024,
                minor: 5,
                patch: 23,
                pre: Prerelease::EMPTY,
                build: BuildMetadata::new("123").unwrap(),
            }
        );
        assert_eq!(ver.to_string(), "2024-05-23.123");

        // pre
        let ver = CalVer::parse("2024-05-1-alpha.1").unwrap();

        assert_eq!(
            ver.0,
            Version {
                major: 2024,
                minor: 5,
                patch: 1,
                pre: Prerelease::new("alpha.1").unwrap(),
                build: BuildMetadata::EMPTY,
            }
        );
        assert_eq!(ver.to_string(), "2024-05-01-alpha.1");
    }
}
