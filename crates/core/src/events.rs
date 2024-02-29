use starbase_events::Event;
use version_spec::*;

macro_rules! impl_event {
    ($name:ident, $impl:tt) => {
        impl_event!($name, (), $impl);
    };

    ($name:ident, $data:ty, $impl:tt) => {
        pub struct $name $impl

        impl Event for $name {
            type Data = $data;
        }
    };
}

impl_event!(InstallingEvent, {
    pub version: VersionSpec,
});

impl_event!(InstalledEvent, {
    pub version: VersionSpec,
});

impl_event!(UninstallingEvent, {
    pub version: VersionSpec,
});

impl_event!(UninstalledEvent, {
    pub version: VersionSpec,
});

impl_event!(UninstalledGlobalEvent, {
    pub dependency: String,
});

impl_event!(CreatedBinariesEvent, {
    pub bins: Vec<String>,
});

impl_event!(CreatedShimsEvent, {
    pub global: Vec<String>,
    pub local: Vec<String>,
});

impl_event!(ResolvedVersionEvent, {
    pub candidate: UnresolvedVersionSpec,
    pub version: VersionSpec,
});
