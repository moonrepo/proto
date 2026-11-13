use crate::AnyResult;
use anyhow::anyhow;
use serde_json::Value;
use std::borrow::Cow;
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

/// Select values from a JSON document using a simplified JSON path.
///
/// The path supports an optional leading `$`, object keys (`key`, `.key`),
/// array indexes (`[0]`), and wildcards (`*`, `.*`, `[*]`), which select
/// every item of an array, or every value of an object. For example,
/// `$.releases[*].version` or `*.version`. A final wildcard followed by
/// `~` (`*~`, `[*]~`) selects the keys of every object instead, for example,
/// `versions.*~`.
///
/// Returns an error if the path is invalid, or if any segment does not match
/// a value (including wildcards that match nothing).
pub fn select_json_path<'a>(root: &'a Value, path: &str) -> AnyResult<Vec<Cow<'a, Value>>> {
    enum Segment {
        Key(String),
        Index(usize),
        Wildcard,
        Keys,
    }

    let mut segments = vec![];
    let mut chars = path.chars().peekable();

    if chars.peek() == Some(&'$') {
        chars.next();
    }

    while let Some(character) = chars.next() {
        match character {
            '.' => {
                let mut key = String::new();

                while let Some(&next) = chars.peek() {
                    if matches!(next, '.' | '[' | ']') {
                        break;
                    }
                    key.push(next);
                    chars.next();
                }

                if key == "*" {
                    segments.push(Segment::Wildcard);
                } else if key == "*~" {
                    segments.push(Segment::Keys);
                } else if !key.is_empty() {
                    segments.push(Segment::Key(key));
                }
            }
            '[' => {
                let mut token = String::new();

                while let Some(&next) = chars.peek() {
                    if next == ']' {
                        break;
                    }
                    token.push(next);
                    chars.next();
                }

                if chars.next() != Some(']') {
                    return Err(anyhow!("Invalid JSON path {path:?}"));
                }

                if token == "*" && chars.peek() == Some(&'~') {
                    chars.next();
                    segments.push(Segment::Keys);
                } else if token == "*" {
                    segments.push(Segment::Wildcard);
                } else {
                    let index = token
                        .parse::<usize>()
                        .map_err(|_| anyhow!("Invalid JSON path {path:?}"))?;
                    segments.push(Segment::Index(index));
                }
            }
            ']' => return Err(anyhow!("Invalid JSON path {path:?}")),
            _ => {
                let mut key = String::from(character);

                while let Some(&next) = chars.peek() {
                    if matches!(next, '.' | '[' | ']') {
                        break;
                    }
                    key.push(next);
                    chars.next();
                }

                if key == "*" {
                    segments.push(Segment::Wildcard);
                } else if key == "*~" {
                    segments.push(Segment::Keys);
                } else {
                    segments.push(Segment::Key(key));
                }
            }
        }
    }

    let keys = matches!(segments.last(), Some(Segment::Keys));

    if keys {
        segments.pop();
    }

    if segments
        .iter()
        .any(|segment| matches!(segment, Segment::Keys))
    {
        return Err(anyhow!("Invalid JSON path {path:?}"));
    }

    let mut current = vec![root];

    for segment in segments {
        let mut next = vec![];

        match segment {
            Segment::Key(key) => {
                for value in current {
                    next.push(
                        value
                            .get(&key)
                            .ok_or_else(|| anyhow!("JSON path {path:?} did not match"))?,
                    );
                }
            }
            Segment::Index(index) => {
                for value in current {
                    next.push(
                        value
                            .as_array()
                            .and_then(|array| array.get(index))
                            .ok_or_else(|| anyhow!("JSON path {path:?} did not match"))?,
                    );
                }
            }
            Segment::Wildcard => {
                for value in current {
                    if let Some(array) = value.as_array() {
                        next.extend(array);
                    } else if let Some(object) = value.as_object() {
                        next.extend(object.values());
                    } else {
                        return Err(anyhow!("JSON path {path:?} did not match"));
                    }
                }
            }
            Segment::Keys => unreachable!(),
        }

        if next.is_empty() {
            return Err(anyhow!("JSON path {path:?} did not match"));
        }

        current = next;
    }

    if !keys {
        return Ok(current.into_iter().map(Cow::Borrowed).collect());
    }

    let mut selected = vec![];

    for value in current {
        let object = value
            .as_object()
            .ok_or_else(|| anyhow!("JSON path {path:?} did not match"))?;

        selected.extend(
            object
                .keys()
                .map(|key| Cow::Owned(Value::String(key.to_owned()))),
        );
    }

    if selected.is_empty() {
        return Err(anyhow!("JSON path {path:?} did not match"));
    }

    Ok(selected)
}
