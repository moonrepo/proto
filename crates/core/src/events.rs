use crate::version::AliasOrVersion;
use starbase_events::Event;

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
    pub version: AliasOrVersion,
});

impl_event!(InstalledEvent, {
    pub version: AliasOrVersion,
});

impl_event!(InstalledGlobalEvent, {
    pub dependency: String,
});

impl_event!(UninstallingEvent, {
    pub version: AliasOrVersion,
});

impl_event!(UninstalledEvent, {
    pub version: AliasOrVersion,
});

impl_event!(UninstalledGlobalEvent, {
    pub dependency: String,
});

impl_event!(CreatedShimsEvent, {
    pub global: Vec<String>,
    pub local: Vec<String>,
});
