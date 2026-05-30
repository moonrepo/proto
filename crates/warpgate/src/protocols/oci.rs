use super::{LoadFrom, LoaderProtocol};
use crate::clients::OciClient;
use crate::loader_error::WarpgateLoaderError;
use crate::registry::*;
use oci_client::{Reference, errors::OciDistributionError};
use starbase_styles::color;
use std::borrow::Cow;
use std::sync::Arc;
use tracing::trace;
use warpgate_api::{Id, RegistryLocator};

#[derive(Clone)]
pub struct OciLoader {
    pub client: Arc<OciClient>,
    pub registries: Vec<RegistryConfig>,
}

impl OciLoader {
    async fn pull_image<'a>(
        &self,
        id: &'a str,
        locator: &'a RegistryLocator,
        config: &RegistryConfig,
        fallthrough: bool,
    ) -> Result<Option<LoadFrom<'a>>, WarpgateLoaderError> {
        let image = locator.image.as_ref();
        let tag = locator.tag.as_deref().unwrap_or("latest");

        let auth = config.get_credential();
        let reference = Reference::try_from(config.get_reference_with_tag(image, tag).as_str())
            .map_err(|error| WarpgateLoaderError::OCIReferenceError {
                message: error.to_string(),
            })?;

        trace!(
            id,
            "Searching OCI registry for {}",
            color::url(reference.to_string())
        );

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
                    WASM_LAYER_MEDIA_TYPE_MARKDOWN,
                    WASM_LAYER_MEDIA_TYPE_TAR,
                    WASM_LAYER_MEDIA_TYPE_TAR_GZIP,
                    WASM_LAYER_MEDIA_TYPE_TAR_ZSTD,
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

        // Find the WASM layer first
        let layer = image_data
            .layers
            .iter()
            .find(|layer| layer.media_type == WASM_LAYER_MEDIA_TYPE_WASM)
            // Otherwise fallback to a non-WASM layer
            .or_else(|| {
                image_data.layers.iter().find(|layer| {
                    layer.media_type == WASM_LAYER_MEDIA_TYPE_TOML
                        || layer.media_type == WASM_LAYER_MEDIA_TYPE_JSON
                        || layer.media_type == WASM_LAYER_MEDIA_TYPE_YAML
                })
            })
            // If still nothing, maybe an archive is being used
            // This happens for certain registries, like GHCR
            .or_else(|| {
                image_data.layers.iter().find(|layer| {
                    layer.media_type == WASM_LAYER_MEDIA_TYPE_TAR
                        || layer.media_type == WASM_LAYER_MEDIA_TYPE_TAR_GZIP
                        || layer.media_type == WASM_LAYER_MEDIA_TYPE_TAR_ZSTD
                })
            });

        Ok(layer.map(|layer| {
            let digest = layer.sha256_digest();

            LoadFrom::Blob {
                data: Cow::Owned(layer.data.to_vec()),
                hash: Cow::Owned(digest.strip_prefix("sha256:").unwrap_or(&digest).into()),
                ext: match layer.media_type.as_str() {
                    WASM_LAYER_MEDIA_TYPE_TOML => "toml",
                    WASM_LAYER_MEDIA_TYPE_YAML => "yaml",
                    WASM_LAYER_MEDIA_TYPE_JSON => "json",
                    _ => "wasm",
                }
                .into(),
                ext_archive: match layer.media_type.as_str() {
                    WASM_LAYER_MEDIA_TYPE_TAR => Some("tar".into()),
                    WASM_LAYER_MEDIA_TYPE_TAR_GZIP => Some("tar.gz".into()),
                    WASM_LAYER_MEDIA_TYPE_TAR_ZSTD => Some("tar.zst".into()),
                    _ => None,
                },
            }
        }))
    }
}

impl LoaderProtocol<RegistryLocator> for OciLoader {
    fn is_latest(&self, locator: &RegistryLocator) -> bool {
        locator.tag.as_ref().is_none_or(|tag| tag == "latest")
    }

    async fn load<'a>(
        &self,
        id: &'a Id,
        locator: &'a RegistryLocator,
    ) -> Result<LoadFrom<'a>, WarpgateLoaderError> {
        trace!(id = id.as_str(), "Loading from OCI registries");

        // If the locator has defined a specific registry (and namespace),
        // attempt to find a matching registry configuration
        if let Some(host) = &locator.registry {
            // 1) Search the configs
            if let Some(registry) = self.registries.iter().find(|registry| {
                // Matches host (always)
                host == &registry.registry
                    // Matches namespace (if specified)
                    && registry
                        .namespace
                        .as_ref()
                        .is_none_or(|ns| {
                            locator.namespace.as_ref().is_some_and(|loc_ns| loc_ns == ns)
                        })
            }) && let Some(from) = self.pull_image(id, locator, registry, true).await?
            {
                return Ok(from);
            }

            // 2) Use an explicit config
            if let Some(from) = self
                .pull_image(
                    id,
                    locator,
                    &RegistryConfig {
                        auth: false,
                        default: false,
                        registry: host.into(),
                        namespace: locator.namespace.clone(),
                    },
                    false,
                )
                .await?
            {
                return Ok(from);
            }
        }

        // Then try all the configured registries
        for registry in &self.registries {
            if let Some(from) = self.pull_image(id, locator, registry, true).await? {
                return Ok(from);
            }
        }

        Err(WarpgateLoaderError::OCIReferenceError {
            message: format!("No valid registry or layer found for {}.", locator.image),
        })
    }
}
