use super::{LoadFrom, LoaderProtocol};
use crate::clients::OciClient;
use crate::id::Id;
use crate::loader_error::WarpgateLoaderError;
use crate::registry::*;
use oci_client::{Reference, errors::OciDistributionError};
use std::sync::Arc;
use tracing::trace;
use warpgate_api::RegistryLocator;

#[derive(Clone)]
pub struct OciLoader {
    pub client: Arc<OciClient>,
}

impl OciLoader {
    async fn pull_image(
        &self,
        id: &str,
        locator: &RegistryLocator,
        config: &RegistryConfig,
        fallthrough: bool,
    ) -> Result<Option<LoadFrom>, WarpgateLoaderError> {
        let image = locator.image.as_ref();
        let tag = locator.tag.as_deref().unwrap_or("latest");

        trace!(
            id,
            "Searching OCI registry {} for {image}:{tag}", config.registry
        );

        let reference =
            Reference::try_from(config.get_reference_with_tag(image, tag).as_str()).unwrap();
        let auth = config.get_credential();

        // Pull the image data and handle the error accordingly
        let image_data = match self
            .client
            .pull(
                &reference,
                &auth,
                vec![
                    WASM_LAYER_MEDIA_TYPE_WASM,
                    WASM_LAYER_MEDIA_TYPE_TOML,
                    WASM_LAYER_MEDIA_TYPE_YAML,
                    WASM_LAYER_MEDIA_TYPE_JSON,
                ],
            )
            .await
        {
            Ok(data) => data,
            Err(error) => {
                return if fallthrough
                    && matches!(
                        error,
                        // Image does not exist in this registry!
                        OciDistributionError::ImageManifestNotFoundError(_)
                    ) {
                    Ok(None)
                } else {
                    Err(WarpgateLoaderError::OciDistributionError {
                        error: Box::new(error),
                        reference: Box::new(reference),
                    })
                };
            }
        };

        // Find the WASM layer first, otherwise fallback to a non-WASM layer
        let layer = image_data
            .layers
            .iter()
            .find(|layer| layer.media_type == WASM_LAYER_MEDIA_TYPE_WASM)
            .or_else(|| {
                image_data.layers.iter().find(|layer| {
                    layer.media_type == WASM_LAYER_MEDIA_TYPE_TOML
                        || layer.media_type == WASM_LAYER_MEDIA_TYPE_JSON
                        || layer.media_type == WASM_LAYER_MEDIA_TYPE_YAML
                })
            });

        Ok(layer.map(|layer| {
            let digest = layer.sha256_digest();

            LoadFrom::Blob {
                data: layer.data.to_owned(),
                hash: digest.strip_prefix("sha256:").unwrap_or(&digest).into(),
                ext: match layer.media_type.as_str() {
                    WASM_LAYER_MEDIA_TYPE_WASM => ".wasm",
                    WASM_LAYER_MEDIA_TYPE_TOML => ".toml",
                    WASM_LAYER_MEDIA_TYPE_YAML => ".yaml",
                    WASM_LAYER_MEDIA_TYPE_JSON => ".json",
                    _ => unreachable!(),
                }
                .into(),
            }
        }))
    }
}

impl LoaderProtocol<RegistryLocator> for OciLoader {
    type Data = Vec<RegistryConfig>;

    fn is_latest(&self, locator: &RegistryLocator) -> bool {
        locator.tag.as_ref().is_none_or(|tag| tag == "latest")
    }

    async fn load(
        &self,
        id: &Id,
        locator: &RegistryLocator,
        data: &Self::Data,
    ) -> Result<LoadFrom, WarpgateLoaderError> {
        trace!(id = id.as_str(), "Scanning and loading from OCI registries");

        // Try the explicit registry first
        if let Some(registry) = &locator.registry
            && let Some(from) = self
                .pull_image(
                    id,
                    locator,
                    &RegistryConfig {
                        registry: registry.to_string(),
                        namespace: locator.namespace.clone(),
                    },
                    false,
                )
                .await?
        {
            return Ok(from);
        }

        // Then try the configured registries
        for registry in data {
            if let Some(from) = self.pull_image(id, locator, registry, true).await? {
                return Ok(from);
            }
        }

        Err(WarpgateLoaderError::OCIReferenceError {
            message: "No valid registry found or no valid layer.".into(),
            location: locator.image.clone(),
        })
    }
}
