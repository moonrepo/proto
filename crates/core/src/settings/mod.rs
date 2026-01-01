#![allow(clippy::module_inception)]

mod backends;
mod plugins;
mod settings;
mod tools;

pub use backends::*;
pub use plugins::*;
pub use settings::*;
pub use tools::*;

use schematic::{Config, ConfigEnum, MergeError, MergeResult, PartialConfig, derive_enum};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum PluginType {
        Backend,
        #[default]
        Tool,
    }
);

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    #[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
    pub enum ConfigMode {
        Global,
        Local,
        Upwards,
        #[default]
        #[serde(alias = "all")]
        #[cfg_attr(feature = "clap", value(alias("all")))]
        UpwardsGlobal,
    }
);

impl ConfigMode {
    pub fn includes_global(&self) -> bool {
        matches!(self, Self::Global | Self::UpwardsGlobal)
    }

    pub fn only_global(&self) -> bool {
        matches!(self, Self::Global)
    }

    pub fn only_local(&self) -> bool {
        matches!(self, Self::Local)
    }
}

derive_enum!(
    #[derive(ConfigEnum, Default)]
    pub enum DetectStrategy {
        #[default]
        FirstAvailable,
        PreferPrototools,
        OnlyPrototools,
    }
);

derive_enum!(
    #[derive(Copy, ConfigEnum, Default)]
    #[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
    pub enum PinLocation {
        #[serde(alias = "store")]
        #[cfg_attr(feature = "clap", value(alias("store")))]
        Global,
        #[default]
        #[serde(alias = "cwd")]
        #[cfg_attr(feature = "clap", value(alias("cwd")))]
        Local,
        #[serde(alias = "home")]
        #[cfg_attr(feature = "clap", value(alias("home")))]
        User,
    }
);

#[derive(Clone, Debug, PartialEq)]
pub struct EnvFile {
    pub path: PathBuf,
    pub weight: usize,
}

#[derive(Clone, Config, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum EnvVar {
    State(bool),
    Value(String),
}

impl EnvVar {
    pub fn to_value(&self) -> Option<String> {
        match self {
            Self::State(state) => state.then(|| "true".to_owned()),
            Self::Value(value) => Some(value.to_owned()),
        }
    }
}

pub(crate) fn merge_iter<I, V>(mut prev: I, next: I, _context: &()) -> MergeResult<I>
where
    I: IntoIterator<Item = V> + Extend<V>,
{
    prev.extend(next);

    Ok(Some(prev))
}

pub(crate) fn merge_partials_iter<K, V>(
    mut prev: BTreeMap<K, V>,
    next: BTreeMap<K, V>,
    context: &V::Context,
) -> MergeResult<BTreeMap<K, V>>
where
    K: Ord,
    V: Default + PartialConfig,
{
    for (key, value) in next {
        prev.entry(key)
            .or_default()
            .merge(context, value)
            .map_err(MergeError::new)?;
    }

    Ok(Some(prev))
}
