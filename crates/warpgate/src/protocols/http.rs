use super::{LoadFrom, LoaderProtocol};
use crate::id::Id;
use crate::loader_error::WarpgateLoaderError;
use tracing::trace;
use warpgate_api::UrlLocator;

#[derive(Clone)]
pub struct HttpLoader {}

impl LoaderProtocol<UrlLocator> for HttpLoader {
    type Data = ();

    fn is_latest(&self, locator: &UrlLocator) -> bool {
        locator.url.contains("latest")
    }

    async fn load(
        &self,
        id: &Id,
        locator: &UrlLocator,
        _: &Self::Data,
    ) -> Result<LoadFrom, WarpgateLoaderError> {
        let url = locator.url.clone();

        trace!(id = id.as_str(), from = &url, "Downloading plugin from URL");

        Ok(LoadFrom::Url(url))
    }
}
