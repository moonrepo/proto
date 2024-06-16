use version_spec::{clean_version_req_string, clean_version_string, is_alias_name};

#[test]
fn checks_alias() {
    assert!(is_alias_name("foo"));
    assert!(is_alias_name("foo.bar"));
    assert!(is_alias_name("foo/bar"));
    assert!(is_alias_name("foo-bar"));
    assert!(is_alias_name("foo_bar-baz"));
    assert!(is_alias_name("alpha.1"));
    assert!(is_alias_name("beta-0"));
    assert!(is_alias_name("rc-1.2.3"));
    assert!(is_alias_name("next-2023"));

    assert!(!is_alias_name("1.2.3"));
    assert!(!is_alias_name("1.2"));
    assert!(!is_alias_name("1"));
    assert!(!is_alias_name("1-3"));
}

#[test]
fn cleans_version() {
    assert_eq!(clean_version_string("1.2.3"), "1.2.3");
    assert_eq!(clean_version_string("v1.2.3"), "1.2.3");
    assert_eq!(clean_version_string("V1.2.3"), "1.2.3");
}

#[test]
fn cleans_req() {
    assert_eq!(clean_version_req_string("1.2.*"), "1.2");
    assert_eq!(clean_version_req_string("1.*.*"), "1");

    assert_eq!(clean_version_req_string("1-2-*"), "1-2");
    assert_eq!(clean_version_req_string("1-*-*"), "1");

    assert_eq!(clean_version_req_string("1 && 2"), "1 , 2");
}
