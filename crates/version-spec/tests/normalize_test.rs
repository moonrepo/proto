use version_spec::{
    FormatOptions, FormatsVersion, UnresolvedVersionSpec, Version, normalize_version,
};

mod normalize_version {
    use super::*;

    #[test]
    fn separates_attached_prereleases() {
        for (input, expected) in [
            ("3.15.0rc2", "3.15.0-rc.2"),
            ("3.15.0a1", "3.15.0-a.1"),
            ("3.15.0b4", "3.15.0-b.4"),
            ("2.6.0c1", "2.6.0-c.1"),
            ("1.0.0alpha1", "1.0.0-alpha.1"),
            ("1.0.0beta12", "1.0.0-beta.12"),
            ("1.0.0dev3", "1.0.0-dev.3"),
            // The number is kept as written
            ("1.0.0rc01", "1.0.0-rc.01"),
        ] {
            assert_eq!(normalize_version(input), expected, "input: {input}");
        }
    }

    #[test]
    fn fills_parts_for_prereleases() {
        assert_eq!(normalize_version("3.0rc1"), "3.0.0-rc.1");
        assert_eq!(normalize_version("3rc1"), "3.0.0-rc.1");

        // But not without a pre-release, as a partial is a requirement
        assert_eq!(normalize_version("3.12"), "3.12");
        assert_eq!(normalize_version("3"), "3");
    }

    #[test]
    fn removes_leading_zeros() {
        assert_eq!(normalize_version("2021.01.07"), "2021.1.7");
        assert_eq!(normalize_version("00.010.000"), "0.10.0");
        assert_eq!(normalize_version("03.012"), "3.12");
        assert_eq!(normalize_version("2021.01.07rc1"), "2021.1.7-rc.1");
        assert_eq!(normalize_version("1.02.3-rc1"), "1.2.3-rc1");
    }

    #[test]
    fn keeps_prefix_and_build() {
        assert_eq!(normalize_version("v3.15.0rc2"), "v3.15.0-rc.2");
        assert_eq!(normalize_version("V1.02.3"), "V1.2.3");
        assert_eq!(
            normalize_version("3.15.0rc2+20260901"),
            "3.15.0-rc.2+20260901"
        );
        assert_eq!(normalize_version("1.02.3+build.01"), "1.2.3+build.01");
        assert_eq!(normalize_version("  3.15.0rc2  "), "3.15.0-rc.2");
    }

    #[test]
    fn keeps_values_that_already_parse() {
        for input in [
            "1.2.3",
            "1.2.3-rc1",
            "1.2.3-rc.1",
            "1.2.3-a.1",
            "1.2.3-0",
            "1.2.3+build",
            "v1.2.3",
            "1.2",
            "2024-01-05",
            "2024-01",
            "node-1.2.3",
            ">=1.2.3",
            "^1.2 || ~2",
            "*",
            "latest",
            "lts-hydrogen",
            "canary",
        ] {
            assert_eq!(normalize_version(input), input, "input: {input}");
        }
    }

    #[test]
    fn keeps_unsupported_shapes() {
        for input in [
            // A label without a number, or in another case
            "1.0.0rc",
            "1.0.0RC1",
            // Other separators, which wouldn't convert back as written
            "1.0.0.rc1",
            "1.0.0_rc1",
            "1.0.dev1",
            // Labels that aren't pre-releases, or more than one label
            "1.0.0post1",
            "3.13.0t",
            "1.0.0a1.dev2",
            "1.0.0rc1-1",
            "",
        ] {
            assert_eq!(normalize_version(input), input, "input: {input}");
        }
    }

    #[test]
    fn is_idempotent() {
        for input in [
            "3.15.0rc2",
            "3.0rc1",
            "2021.01.07",
            "v1.0.0dev3+build",
            "1.02.3-rc1",
        ] {
            let once = normalize_version(input);

            assert_eq!(normalize_version(&once), once, "input: {input}");
        }
    }

    #[test]
    fn parses_strictly_after() {
        for (input, expected) in [
            ("3.15.0rc2", "3.15.0-rc.2"),
            ("3.0rc1", "3.0.0-rc.1"),
            ("v1.0.0alpha1", "1.0.0-alpha.1"),
            ("2021.01.07", "2021.1.7"),
        ] {
            assert!(Version::parse(input).is_err(), "input: {input}");
            assert_eq!(
                Version::parse(normalize_version(input))
                    .unwrap()
                    .to_string(),
                expected,
                "input: {input}"
            );
        }

        // A partial parses as a requirement
        assert_eq!(
            UnresolvedVersionSpec::parse(normalize_version("3.012")).unwrap(),
            UnresolvedVersionSpec::parse("3.12").unwrap()
        );
    }

    #[test]
    fn orders_numerically() {
        let rc2 = Version::parse(normalize_version("1.0.0rc2")).unwrap();
        let rc10 = Version::parse(normalize_version("1.0.0rc10")).unwrap();

        assert!(rc2 < rc10);
        assert!(rc10 < Version::parse("1.0.0").unwrap());
    }
}

mod attach_prerelease {
    use super::*;

    fn attached() -> FormatOptions {
        FormatOptions {
            attach_prerelease: true,
            ..Default::default()
        }
    }

    #[test]
    fn converts_back_to_the_original() {
        for input in [
            "3.15.0rc2",
            "3.15.0a1",
            "1.0.0beta12",
            "1.0.0dev3",
            "v3.15.0rc2",
            "3.15.0rc2+20260901",
        ] {
            let version = Version::parse(normalize_version(input)).unwrap();

            assert_eq!(
                version.to_formatted_string(&attached()),
                input.trim_start_matches('v'),
                "input: {input}"
            );
        }
    }

    #[test]
    fn converts_back_with_padding() {
        let version = Version::parse(normalize_version("2021.01.07")).unwrap();

        assert_eq!(
            version.to_formatted_string(&FormatOptions {
                pad_minor: Some(2),
                pad_patch: Some(2),
                ..attached()
            }),
            "2021.01.07"
        );
    }

    #[test]
    fn keeps_the_rest_of_the_prerelease() {
        for (input, expected) in [
            ("1.2.3-rc.2", "1.2.3rc2"),
            ("1.2.3-rc2", "1.2.3rc2"),
            ("1.2.3-rc", "1.2.3rc"),
            ("1.2.3-rc.2.1", "1.2.3rc2.1"),
            ("1.2.3-rc.beta", "1.2.3rc.beta"),
            ("1.2.3-rc-1", "1.2.3rc-1"),
            // A pre-release that doesn't start with a letter keeps the separator
            ("1.2.3-0", "1.2.3-0"),
            ("1.2.3-1.rc", "1.2.3-1.rc"),
            ("1.2.3--canary.9", "1.2.3--canary.9"),
        ] {
            assert_eq!(
                Version::parse(input)
                    .unwrap()
                    .to_formatted_string(&attached()),
                expected,
                "input: {input}"
            );
        }
    }

    #[test]
    fn applies_to_requirements() {
        let req = UnresolvedVersionSpec::parse(">=3.15.0-rc.2").unwrap();

        let UnresolvedVersionSpec::Requirement(req) = req else {
            panic!("expected a requirement");
        };

        assert_eq!(req.to_formatted_string(&attached()), ">=3.15.0rc2");
    }

    #[test]
    fn is_off_by_default() {
        assert_eq!(
            Version::parse("3.15.0-rc.2")
                .unwrap()
                .to_formatted_string(&FormatOptions::default()),
            "3.15.0-rc.2"
        );
    }
}
