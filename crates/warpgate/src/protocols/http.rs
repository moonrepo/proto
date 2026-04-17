use super::{LoadFrom, LoaderProtocol};
use crate::loader_error::WarpgateLoaderError;
use std::borrow::Cow;
use tracing::trace;
use warpgate_api::{Id, UrlLocator};

#[derive(Clone)]
pub struct HttpLoader {}

impl LoaderProtocol<UrlLocator> for HttpLoader {
    type Data = ();

    fn is_latest(&self, locator: &UrlLocator) -> bool {
        locator.url.contains("latest")
    }

    async fn load<'a>(
        &self,
        id: &'a Id,
        locator: &'a UrlLocator,
        _: &Self::Data,
    ) -> Result<LoadFrom<'a>, WarpgateLoaderError> {
        let url = &locator.url;

        trace!(id = id.as_str(), from = url, "Downloading plugin from URL");

        Ok(LoadFrom::Url(Cow::Borrowed(url)))
    }
}
