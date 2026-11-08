use std::mem::size_of;
use version_spec::*;

// These types are stored in bulk (a tool's full version list) and scanned
// linearly during resolution, so guard against silent size regressions.
#[test]
fn types_do_not_grow() {
    assert_eq!(size_of::<Version>(), 96);
    assert_eq!(size_of::<Requirement>(), 88);
    assert_eq!(size_of::<Clause>(), 88);
    assert_eq!(size_of::<Range>(), 24);
    assert_eq!(size_of::<VersionSpec>(), 96);
    assert_eq!(size_of::<UnresolvedVersionSpec>(), 96);
}
