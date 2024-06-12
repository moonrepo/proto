use version_spec::is_calver_like;

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
            assert!(is_calver_like(format!("{year}")));
            assert!(is_calver_like(format!("{year}-alpha")));
            assert!(is_calver_like(format!("{year}_789")));
            assert!(is_calver_like(format!("{year}.789-alpha")));

            for month in months {
                assert!(is_calver_like(format!("{year}-{month}")));
                assert!(is_calver_like(format!("{year}-{month}-rc.1")));
                assert!(is_calver_like(format!("{year}-{month}_456")));
                assert!(is_calver_like(format!("{year}-{month}_456-rc.2")));
                assert!(is_calver_like(format!("{year}-{month}.456")));

                for day in days {
                    assert!(is_calver_like(format!("{year}-{month}-{day}")));
                    assert!(is_calver_like(format!("{year}-{month}-{day}-beta.1")));
                    assert!(is_calver_like(format!("{year}-{month}-{day}_123")));
                    assert!(is_calver_like(format!("{year}-{month}-{day}.123")));
                    assert!(is_calver_like(format!("{year}-{month}-{day}.123-beta.1")));
                }
            }
        }
    }

    #[test]
    fn doesnt_match() {
        // invalid months
        assert!(!is_calver_like("2024-0"));
        assert!(!is_calver_like("2024-00"));
        assert!(!is_calver_like("2024-13"));
        assert!(!is_calver_like("2024-20"));
        assert!(!is_calver_like("2024-010"));

        // invalid days
        assert!(!is_calver_like("2024-10-0"));
        assert!(!is_calver_like("2024-10-00"));
        assert!(!is_calver_like("2024-10-123"));
        assert!(!is_calver_like("2024-10-40"));
        assert!(!is_calver_like("2024-10-50"));

        // invalid micro
        assert!(!is_calver_like("2024_abc"));
        assert!(!is_calver_like("2024-10_abc"));
        assert!(!is_calver_like("2024-1-1_abc"));
    }
}
