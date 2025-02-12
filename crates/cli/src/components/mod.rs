// mod aliases_map;
mod code_block;
mod env_var;
mod install_all_progress;
mod install_progress;
mod issues_list;
mod locator;
mod spec_aliases_map;
mod versions_map;

// pub use aliases_map::*;
pub use code_block::*;
pub use env_var::*;
pub use install_all_progress::*;
pub use install_progress::*;
pub use issues_list::*;
pub use locator::*;
pub use spec_aliases_map::*;
pub use versions_map::*;

use chrono::{DateTime, NaiveDateTime};

pub fn is_path_like(value: impl AsRef<str>) -> bool {
    let value = value.as_ref();
    value.contains('/') || value.contains("\\")
}

pub fn create_datetime(millis: u128) -> Option<NaiveDateTime> {
    DateTime::from_timestamp((millis / 1000) as i64, ((millis % 1000) * 1_000_000) as u32)
        .map(|dt| dt.naive_local())
}
