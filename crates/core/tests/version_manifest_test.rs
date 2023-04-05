use proto_core::VersionManifest;

#[test]
fn recursively_unwraps_aliases() {
    let mut manifest = VersionManifest::default();
    manifest
        .aliases
        .insert("first".to_owned(), "second".to_owned());
    manifest
        .aliases
        .insert("second".to_owned(), "third".to_owned());
    manifest
        .aliases
        .insert("third".to_owned(), "1.2.3".to_owned());

    assert_eq!(manifest.get_version_from_alias("first").unwrap(), "1.2.3");
}
