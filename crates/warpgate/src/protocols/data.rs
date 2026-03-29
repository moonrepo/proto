use super::{LoadFrom, LoaderProtocol};
use crate::helpers::create_cache_key;
use crate::loader_error::WarpgateLoaderError;
use base64::prelude::*;
use tracing::trace;
use warpgate_api::{DataLocator, Id};

#[derive(Clone)]
pub struct DataLoader {}

impl LoaderProtocol<DataLocator> for DataLoader {
    type Data = ();

    fn is_latest(&self, _locator: &DataLocator) -> bool {
        true
    }

    async fn load(
        &self,
        id: &Id,
        locator: &DataLocator,
        _: &Self::Data,
    ) -> Result<LoadFrom, WarpgateLoaderError> {
        trace!(id = id.as_str(), "Linking plugin from explicit byte stream");

        let encoded_data = locator
            .data
            .strip_prefix("data://")
            .unwrap_or(&locator.data);

        let data = match &locator.bytes {
            Some(bytes) => bytes.clone(),
            None => BASE64_STANDARD.decode(encoded_data).map_err(|error| {
                WarpgateLoaderError::Base64DecodeError {
                    error: Box::new(error),
                }
            })?,
        };

        Ok(LoadFrom::Blob {
            hash: create_cache_key(encoded_data, None),
            ext: ".wasm".into(),
            data,
        })
    }
}
