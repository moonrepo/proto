mod file;
mod github;
mod http;
mod oci;

pub use file::*;
pub use github::*;
pub use http::*;
pub use oci::*;

use crate::loader_error::WarpgateLoaderError;
use starbase_id::Id;
use std::path::PathBuf;

pub trait LoaderProtocol<T> {
    type Data;

    fn is_latest(&self, locator: &T) -> bool;

    async fn load(
        &self,
        id: &Id,
        locator: &T,
        data: &Self::Data,
    ) -> Result<LoadFrom, WarpgateLoaderError>;
}

pub enum LoadFrom {
    Blob {
        data: Vec<u8>,
        ext: String,
        hash: String,
    },
    File(PathBuf),
    Url(String),
}
