use proto_pdk_api::{extract_version_from_file, extract_version_from_text};
use starbase_sandbox::create_empty_sandbox;

mod extract_version_from_file {
    use super::*;

    #[test]
    fn matches_extracting_from_text() {
        let sandbox = create_empty_sandbox();
        let path = sandbox.path().join(".version");

        for content in [
            "20.18.0",
            "  lts/*  \n",
            "\n\n20\n22\n",
            "# pinned for CI\n// pinned for CI\n20",
            "20 # LTS",
            "20 // LTS",
            "\t3.12.1\r\n",
            "",
            "  \n\n",
            "# only a comment",
        ] {
            std::fs::write(&path, content).unwrap();

            assert_eq!(
                extract_version_from_file(&path).unwrap().as_deref(),
                extract_version_from_text(content),
                "for {content:?}"
            );
        }
    }

    #[test]
    fn stops_reading_after_a_version() {
        let sandbox = create_empty_sandbox();
        let path = sandbox.path().join(".version");

        std::fs::write(&path, b"1.2.3\n\xFF\xFE").unwrap();

        assert_eq!(
            extract_version_from_file(&path).unwrap().as_deref(),
            Some("1.2.3")
        );
    }

    #[test]
    fn errors_for_invalid_utf8_before_a_version() {
        let sandbox = create_empty_sandbox();
        let path = sandbox.path().join(".version");

        std::fs::write(&path, b"\xFF\xFE\n1.2.3").unwrap();

        assert!(extract_version_from_file(&path).is_err());
    }

    #[test]
    fn errors_for_missing_file() {
        let sandbox = create_empty_sandbox();

        assert!(extract_version_from_file(sandbox.path().join(".version")).is_err());
    }
}

mod extract_version_from_text {
    use super::*;

    #[test]
    fn extracts_first_version_line() {
        for (content, expected) in [
            ("20.18.0", "20.18.0"),
            ("  lts/*  \n", "lts/*"),
            ("\n\n20\n22\n", "20"),
            ("# pinned for CI\n20", "20"),
            ("// pinned for CI\n20", "20"),
            ("20 # LTS", "20"),
            ("20#LTS\n22", "20"),
            ("20 // LTS", "20"),
            ("20//LTS\n22", "20"),
            ("20 // a # b", "20"),
            ("20 # a // b", "20"),
            ("  // indented\n  # indented\n20", "20"),
            ("lts/*", "lts/*"),
            ("lts/* // LTS", "lts/*"),
            (">=18 <20 # range", ">=18 <20"),
            ("\t3.12.1\r\n", "3.12.1"),
        ] {
            assert_eq!(
                extract_version_from_text(content),
                Some(expected),
                "for {content:?}"
            );
        }
    }

    #[test]
    fn extracts_nothing_without_a_version() {
        for content in [
            "",
            "  \n\n",
            "# only a comment",
            "// only a comment",
            "  # indented\n  // indented",
        ] {
            assert_eq!(extract_version_from_text(content), None, "for {content:?}");
        }
    }
}
