use std::mem::size_of;
use version_spec::*;

// These types are stored in bulk (a tool's full version list) and scanned
// linearly during resolution, so guard against silent size regressions.
#[test]
fn types_do_not_grow() {
    assert_eq!(size_of::<Version>(), 48);
    assert_eq!(size_of::<Requirement>(), 56);
    assert_eq!(size_of::<Clause>(), 56);
    assert_eq!(size_of::<Range>(), 24);
    assert_eq!(size_of::<VersionSpec>(), 48);
    assert_eq!(size_of::<UnresolvedVersionSpec>(), 56);
}

// Rarely defined, so an undefined string must not cost more than a pointer
#[test]
fn shared_strings_are_a_pointer() {
    assert_eq!(size_of::<SharedString>(), size_of::<usize>());
    assert_eq!(size_of::<Option<SharedString>>(), size_of::<usize>());
}
