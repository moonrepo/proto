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
    fn is_latest(&self, locator: &T) -> bool;

    fn requires_online(&self) -> bool {
        true
    }

    async fn load<'a>(
        &self,
        id: &'a Id,
        locator: &'a T,
    ) -> Result<LoadFrom<'a>, WarpgateLoaderError>;
}

pub enum LoadFrom<'a> {
    Blob {
        data: Cow<'a, [u8]>,
        ext: String,
        ext_archive: Option<String>,

        #[allow(dead_code)]
        hash: Cow<'a, str>,
    },
    File(PathBuf),
    Url(Cow<'a, str>),
}

impl LoadFrom<'_> {
    pub fn is_archive(&self) -> Option<String> {
        match self {
            LoadFrom::Blob { ext_archive, .. } => ext_archive.clone(),
            LoadFrom::File(_) => None,
            LoadFrom::Url(url) => {
                let file_name = extract_file_name_from_url(url);

                get_supported_archive_extensions()
                    .into_iter()
                    .find(|ext| file_name.ends_with(format!(".{ext}").as_str()))
                    .map(|ext| ext.to_owned())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(ext_archive: Option<&str>) -> LoadFrom<'static> {
        LoadFrom::Blob {
            data: Cow::Owned(vec![]),
            ext: "wasm".into(),
            ext_archive: ext_archive.map(|s| s.to_owned()),
            hash: Cow::Owned(String::new()),
        }
    }

    fn url(value: &str) -> LoadFrom<'_> {
        LoadFrom::Url(Cow::Borrowed(value))
    }

    mod is_archive {
        use super::*;

        // -- Blob variants ----------------------------------------------------

        // OCI tar layers populate `ext_archive`. The method should pass the
        // value through untouched so the loader knows the temp file needs the
        // archive extension before unpacking.
        #[test]
        fn blob_with_archive_ext_is_returned_verbatim() {
            assert_eq!(blob(Some("tar.gz")).is_archive(), Some("tar.gz".into()));
            assert_eq!(blob(Some("tar.zst")).is_archive(), Some("tar.zst".into()));
            assert_eq!(blob(Some("tar")).is_archive(), Some("tar".into()));
        }

        // Plain WASM blobs (no archive layer) must report no archive so the
        // loader skips the unpack step.
        #[test]
        fn blob_without_archive_ext_returns_none() {
            assert_eq!(blob(None).is_archive(), None);
        }

        // -- File variants ---------------------------------------------------

        // File locators bypass caching/unpacking entirely, so `is_archive` must
        // never claim a file is an archive (regardless of its extension on disk).
        #[test]
        fn file_always_returns_none() {
            assert_eq!(
                LoadFrom::File(PathBuf::from("/tmp/whatever.tar.gz")).is_archive(),
                None
            );
            assert_eq!(
                LoadFrom::File(PathBuf::from("/tmp/plugin.wasm")).is_archive(),
                None
            );
        }

        // -- Url variants ----------------------------------------------------

        // URL detection uses `PathBuf::extension`, which only sees the LAST
        // extension component. The matching list from `starbase_archive`
        // includes single extensions like `gz`/`zst`/`zip`, so compound names
        // still resolve through that single suffix.
        #[test]
        fn url_with_supported_archive_extension() {
            assert_eq!(
                url("https://example.com/path/foo.tar.gz").is_archive(),
                Some("tar.gz".into())
            );
            assert_eq!(
                url("https://example.com/path/foo.tar.zst").is_archive(),
                Some("tar.zst".into())
            );
            assert_eq!(
                url("https://example.com/path/foo.zip").is_archive(),
                Some("zip".into())
            );
            assert_eq!(
                url("https://example.com/path/foo.tar").is_archive(),
                Some("tar".into())
            );
        }

        // WASM is not an archive — extension must be rejected even though it's
        // a familiar plugin payload.
        #[test]
        fn url_with_wasm_extension_returns_none() {
            assert_eq!(url("https://example.com/foo.wasm").is_archive(), None);
        }

        // Unsupported extensions and URLs without any extension must all
        // resolve to `None` so the loader treats them as plain downloads.
        #[test]
        fn url_with_unsupported_or_missing_extension() {
            assert_eq!(url("https://example.com/foo.7z").is_archive(), None);
            assert_eq!(url("https://example.com/foo.txt").is_archive(), None);
            assert_eq!(url("https://example.com/foo").is_archive(), None);
            assert_eq!(url("https://example.com/").is_archive(), None);
        }
    }
}
