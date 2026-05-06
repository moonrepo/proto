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

use crate::helpers::extract_file_name_from_url;
use crate::loader_error::WarpgateLoaderError;
use starbase_archive::get_supported_archive_extensions;
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
        archive: bool,
        data: Cow<'a, [u8]>,
        ext: String,
        hash: Cow<'a, str>,
    },
    File(PathBuf),
    Url(Cow<'a, str>),
}

impl LoadFrom<'_> {
    pub fn is_archive(&self) -> Option<String> {
        match self {
            LoadFrom::Blob { archive, ext, .. } => archive.then(|| ext.into()),
            LoadFrom::File(_) => None,
            LoadFrom::Url(url) => PathBuf::from(extract_file_name_from_url(url))
                .extension()
                .and_then(|ext| ext.to_str())
                .and_then(|ext| {
                    if get_supported_archive_extensions()
                        .iter()
                        .any(|supported| ext == supported)
                    {
                        Some(ext.into())
                    } else {
                        None
                    }
                }),
        }
    }
}
