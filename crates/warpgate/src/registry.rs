use docker_credential::{CredentialRetrievalError, DockerCredential};
use oci_client::secrets::RegistryAuth;
use serde::{Deserialize, Serialize};
use tracing::{trace, warn};

pub const WASM_LAYER_MEDIA_TYPE_WASM: &str = "application/wasm";
pub const WASM_LAYER_MEDIA_TYPE_TOML: &str = "application/toml";
pub const WASM_LAYER_MEDIA_TYPE_JSON: &str = "application/json";
pub const WASM_LAYER_MEDIA_TYPE_YAML: &str = "application/yaml";
pub const WASM_LAYER_MEDIA_TYPE_MARKDOWN: &str = "text/markdown";

pub const WASM_LAYER_MEDIA_TYPE_TAR: &str = "application/vnd.oci.image.layer.v1.tar";
pub const WASM_LAYER_MEDIA_TYPE_TAR_GZIP: &str = "application/vnd.oci.image.layer.v1.tar+gzip";
pub const WASM_LAYER_MEDIA_TYPE_TAR_ZSTD: &str = "application/vnd.oci.image.layer.v1.tar+zstd";

/// Configures an individual plugin registry.
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
pub struct RegistryConfig {
    // Whether this registry requires authentication or not.
    // If true, we'll attempt to retrieve credentials from the Docker
    // config for this registry's host.
    pub auth: bool,

    /// The domain/host of the registry.
    pub registry: String,

    /// An optional namespace to bucket the plugin into.
    /// This is typically the organization or user.
    pub namespace: Option<String>,
}

impl RegistryConfig {
    /// Return the Docker credential's for the current registry host.
    pub fn get_credential(&self) -> RegistryAuth {
        if !self.auth {
            return RegistryAuth::Anonymous;
        }

        match docker_credential::get_credential(&self.registry) {
            Ok(DockerCredential::UsernamePassword(username, password)) => {
                trace!("Found Docker credentials (username and password)");

                RegistryAuth::Basic(username, password)
            }
            Ok(DockerCredential::IdentityToken(_)) => {
                trace!(
                    "Cannot use contents of Docker config, identity token not supported; using anonymous auth"
                );

                RegistryAuth::Anonymous
            }
            Err(CredentialRetrievalError::ConfigNotFound) => RegistryAuth::Anonymous,
            Err(CredentialRetrievalError::NoCredentialConfigured) => RegistryAuth::Anonymous,
            Err(error) => {
                warn!("Error handling Docker configuration file: {error}; using anonymous auth",);

                RegistryAuth::Anonymous
            }
        }
    }

    /// Return a fully-qualified reference with the provided ID.
    pub fn get_reference(&self, id: &str) -> String {
        let mut reference = String::new();
        reference.push_str(&self.registry);
        reference.push('/');

        if let Some(namespace) = &self.namespace {
            reference.push_str(namespace);
            reference.push('/');
        }

        reference.push_str(id);
        reference
    }

    /// Return a fully-qualified reference with the provided ID and tag.
    pub fn get_reference_with_tag(&self, id: &str, tag: &str) -> String {
        let mut reference = self.get_reference(id);
        reference.push(':');
        reference.push_str(tag);
        reference
    }
}
