mod data;
mod file;
mod github;
mod http;
mod oci;

pub use data::*;
pub use file::*;
pub use github::*;
pub use http::*;
pub use oci::*;

use crate::loader_error::WarpgateLoaderError;
use std::borrow::Cow;
use std::path::PathBuf;
use warpgate_api::Id;

pub trait LoaderProtocol<T> {
    type Data;

    fn is_latest(&self, locator: &T) -> bool;

    async fn load<'a>(
        &self,
        id: &'a Id,
        locator: &'a T,
        data: &Self::Data,
    ) -> Result<LoadFrom<'a>, WarpgateLoaderError>;
}

pub enum LoadFrom<'a> {
    Blob {
        data: Cow<'a, [u8]>,
        ext: String,
        hash: Cow<'a, str>,
    },
    File(PathBuf),
    Url(Cow<'a, str>),
}
