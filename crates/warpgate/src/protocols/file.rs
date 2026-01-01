use super::{LoadFrom, LoaderProtocol};
use crate::loader_error::WarpgateLoaderError;
use tracing::trace;
use warpgate_api::{FileLocator, Id};

#[derive(Clone)]
pub struct FileLoader {}

impl LoaderProtocol<FileLocator> for FileLoader {
    type Data = ();

    fn is_latest(&self, _locator: &FileLocator) -> bool {
        true
    }

    async fn load(
        &self,
        id: &Id,
        locator: &FileLocator,
        _: &Self::Data,
    ) -> Result<LoadFrom, WarpgateLoaderError> {
        let path = locator.get_resolved_path();

        if path.exists() {
            trace!(
                id = id.as_str(),
                path = ?path,
                "Linking plugin from local file",
            );

            Ok(LoadFrom::File(path))
        } else {
            Err(WarpgateLoaderError::MissingSourceFile {
                id: id.to_owned(),
                path: path.to_path_buf(),
            })
        }
    }
}
