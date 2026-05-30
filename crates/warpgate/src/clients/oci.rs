use super::http::HttpOptions;
use super::http_error::WarpgateHttpClientError;
use oci_client::Client;
use oci_client::client::{Certificate, CertificateEncoding, ClientConfig};
use starbase_utils::{envx, fs};
use std::ops::Deref;
use std::time::Duration;
use tracing::{debug, trace, warn};

/// An OCI client that wraps [`oci_client::Client`].
#[derive(Clone)]
pub struct OciClient(Client);

impl Deref for OciClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Create an OCI client with the provided options, that'll be
/// used for interacting with OCI registries.
pub fn create_oci_client_with_options(
    options: &HttpOptions,
) -> Result<OciClient, WarpgateHttpClientError> {
    debug!("Creating OCI client");

    let mut config = ClientConfig::default();

    if !envx::bool_var("WARPGATE_OCI_NO_TIMEOUTS") {
        config.read_timeout = Some(Duration::from_mins(5));
        config.connect_timeout = Some(Duration::from_mins(1));
    }

    if options.allow_invalid_certs {
        trace!("Allowing invalid certificates (I hope you know what you're doing!)");

        config.accept_invalid_certificates = options.allow_invalid_certs;
    }

    if let Some(root_cert) = &options.root_cert {
        trace!(root_cert = ?root_cert, "Adding user provided root certificate");

        match root_cert.extension().and_then(|ext| ext.to_str()) {
            Some("der" | "DER") => {
                config.extra_root_certificates.push(Certificate {
                    encoding: CertificateEncoding::Der,
                    data: fs::read_file_bytes(root_cert)?,
                });
            }
            Some("pem" | "PEM") => {
                config.extra_root_certificates.push(Certificate {
                    encoding: CertificateEncoding::Pem,
                    data: fs::read_file_bytes(root_cert)?,
                });
            }
            _ => {
                warn!(
                    root_cert = ?root_cert,
                    "Invalid root certificate type, must be a DER or PEM file",
                );
            }
        };
    }

    debug!("Created OCI client");

    Ok(OciClient(Client::new(config)))
}
