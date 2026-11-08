use std::hash::{DefaultHasher, Hash, Hasher};

/// Create a key for an Extism `var` from a name and the inputs its value depends on.
pub fn create_var_key(name: &str, inputs: impl Hash) -> String {
    create_var_key_with_hasher(name, |hasher| inputs.hash(hasher))
}

/// Create a key for an Extism `var` from a name and a custom hashing operation.
pub fn create_var_key_with_hasher(name: &str, op: impl FnOnce(&mut DefaultHasher)) -> String {
    let mut hasher = DefaultHasher::new();

    op(&mut hasher);

    format!("{name}:{:016x}", hasher.finish())
}
