use compact_str::CompactString;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

/// An immutable string that is cheap to clone, used for the scope,
/// pre-release, and build metadata of versions and requirements.
///
/// Versions are stored in bulk, but these strings are rarely defined, so
/// it's a single pointer, and `Option<SharedString>` is the same size.
/// Strings that fit inline in a [`CompactString`] (up to 24 bytes on 64-bit
/// targets) are stored in the same allocation as the reference count,
/// while longer strings require another allocation.
///
/// It dereferences to [`str`], and compares, orders, and hashes like one.
#[derive(Clone)]
pub struct SharedString(Arc<CompactString>);

impl SharedString {
    /// Creates a shared string from the provided value.
    pub fn new(value: impl AsRef<str>) -> Self {
        Self(Arc::new(CompactString::new(value)))
    }

    /// Returns the string as a slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for SharedString {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for SharedString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for SharedString {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for SharedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for SharedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl PartialEq for SharedString {
    fn eq(&self, other: &Self) -> bool {
        // Clones share the same allocation, so skip comparing them
        Arc::ptr_eq(&self.0, &other.0) || self.as_str() == other.as_str()
    }
}

impl Eq for SharedString {}

impl PartialOrd for SharedString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SharedString {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

// Must hash like `str` to uphold the `Borrow<str>` contract
impl Hash for SharedString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

macro_rules! impl_eq {
    ($($ty:ty),+) => {
        $(
            impl PartialEq<$ty> for SharedString {
                fn eq(&self, other: &$ty) -> bool {
                    self.as_str() == AsRef::<str>::as_ref(other)
                }
            }

            impl PartialEq<SharedString> for $ty {
                fn eq(&self, other: &SharedString) -> bool {
                    AsRef::<str>::as_ref(self) == other.as_str()
                }
            }
        )+
    };
}

impl_eq!(str, &str, String);

// The reverse is implemented by `CompactString` for any `AsRef<str>`
impl PartialEq<CompactString> for SharedString {
    fn eq(&self, other: &CompactString) -> bool {
        self.as_str() == other.as_str()
    }
}

impl From<&str> for SharedString {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<&String> for SharedString {
    fn from(value: &String) -> Self {
        Self::new(value)
    }
}

impl From<String> for SharedString {
    fn from(value: String) -> Self {
        Self(Arc::new(CompactString::from(value)))
    }
}

impl From<CompactString> for SharedString {
    fn from(value: CompactString) -> Self {
        Self(Arc::new(value))
    }
}

impl From<SharedString> for String {
    fn from(value: SharedString) -> Self {
        value.as_str().to_owned()
    }
}

impl From<SharedString> for CompactString {
    fn from(value: SharedString) -> Self {
        Arc::unwrap_or_clone(value.0)
    }
}

impl Serialize for SharedString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SharedString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        CompactString::deserialize(deserializer).map(Self::from)
    }
}
