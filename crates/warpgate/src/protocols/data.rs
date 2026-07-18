use super::{LoadFrom, LoaderProtocol};
use crate::loader_error::WarpgateLoaderError;
use starbase_utils::hash::{self, base64::native::prelude::*};
use std::borrow::Cow;
use tracing::trace;
use warpgate_api::{DataLocator, Id};

#[derive(Clone)]
pub struct DataLoader {}

impl LoaderProtocol<DataLocator> for DataLoader {
    fn is_latest(&self, _locator: &DataLocator) -> bool {
        false
    }

    fn requires_online(&self) -> bool {
        false
    }

    async fn load<'a>(
        &self,
        id: &'a Id,
        locator: &'a DataLocator,
    ) -> Result<LoadFrom<'a>, WarpgateLoaderError> {
        let encoded_data = locator
            .data
            .strip_prefix("data://")
            .unwrap_or(&locator.data);

        let data: Cow<'_, [u8]> = match &locator.bytes {
            Some(bytes) => Cow::Borrowed(bytes),
            None => Cow::Owned(BASE64_STANDARD.decode(encoded_data).map_err(|error| {
                WarpgateLoaderError::Base64DecodeError {
                    error: Box::new(error),
                }
            })?),
        };

        trace!(
            id = id.as_str(),
            size = data.len(),
            "Linking plugin from explicit byte stream"
        );

        Ok(LoadFrom::Blob {
            hash: Cow::Owned(hash::sha256::from_bytes(&*data)),
            ext: "wasm".into(),
            ext_archive: None,
            data,
        })
    }
}
