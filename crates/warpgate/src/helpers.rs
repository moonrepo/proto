pub fn extract_owner_from_slug(slug: &str) -> &str {
    slug.split('/').next().expect("Expected an owner scope!")
}

pub fn extract_item_from_slug(slug: &str) -> &str {
    slug.split('/')
        .nth(1)
        .expect("Expected a package or repository name!")
}

pub fn create_wasm_file_stem(name: &str) -> String {
    let mut name = name.to_lowercase().replace('-', "_");

    if !name.ends_with("_plugin") {
        name.push_str("_plugin");
    }

    name
}
