use version_spec::is_alias_name;

#[test]
fn checks_alias() {
    assert!(is_alias_name("foo"));
    assert!(is_alias_name("foo.bar"));
    assert!(is_alias_name("foo/bar"));
    assert!(is_alias_name("foo-bar"));
    assert!(is_alias_name("foo_bar-baz"));
    assert!(is_alias_name("alpha.1"));
    assert!(is_alias_name("beta-0"));
    assert!(is_alias_name("next-2023"));
    assert!(is_alias_name("ver-2023"));
    assert!(is_alias_name("vue"));

    assert!(!is_alias_name("1.2.3"));
    assert!(!is_alias_name("1.2"));
    assert!(!is_alias_name("1"));
    assert!(!is_alias_name("1-3"));
    assert!(!is_alias_name("v1.2.3"));
    assert!(!is_alias_name("2000-01-01"));
    assert!(!is_alias_name("00-01-01"));
    assert!(!is_alias_name("v00-01-01"));

    // Scoped versions, not aliases
    assert!(!is_alias_name("rc-1.2"));
    assert!(!is_alias_name("node-1.2"));

    // A leading "v" followed by digits is a version prefix
    assert!(!is_alias_name("v8"));
    assert!(!is_alias_name("v10"));
    assert!(!is_alias_name("v10.1"));
}
