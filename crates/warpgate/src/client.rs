use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use starbase_utils::fs;
use std::path::PathBuf;
use tracing::{debug, trace, warn};

/// Configures the HTTPS client used for making requests.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
pub struct HttpOptions {
    /// Allow invalid certificates. This is dangerous and should only be used as a last resort!
    pub allow_invalid_certs: bool,

    /// A list of proxy URLs that all requests should pass through.
    pub proxies: Vec<String>,

    /// Absolute path to the root certificate. Supports `.pem` and `.der` files.
    pub root_cert: Option<PathBuf>,
}

/// Create an HTTP/HTTPS client that'll be used for downloading files.
pub fn create_http_client() -> miette::Result<reqwest::Client> {
    create_http_client_with_options(&HttpOptions::default())
}

/// Create an HTTP/HTTPS client with the provided options, that'll be
/// used for downloading files.
pub fn create_http_client_with_options(options: &HttpOptions) -> miette::Result<reqwest::Client> {
    debug!("Creating HTTP client");

    let mut client = reqwest::Client::builder()
        .user_agent(format!("warpgate@{}", env!("CARGO_PKG_VERSION")))
        .use_rustls_tls();

    if options.allow_invalid_certs {
        trace!("Allowing invalid certificates (I hope you know what you're doing!)");

        client = client.danger_accept_invalid_certs(true);
    }

    if let Some(root_cert) = &options.root_cert {
        trace!(root_cert = ?root_cert, "Adding user provided root certificate");

        match root_cert.extension().and_then(|ext| ext.to_str()) {
            Some("der") => {
                client = client.add_root_certificate(
                    reqwest::Certificate::from_der(&fs::read_file_bytes(root_cert)?)
                        .into_diagnostic()?,
                )
            }
            Some("pem") => {
                client = client.add_root_certificate(
                    reqwest::Certificate::from_pem(&fs::read_file_bytes(root_cert)?)
                        .into_diagnostic()?,
                )
            }
            _ => {
                warn!(
                    root_cert = ?root_cert,
                    "Invalid root certificate type, must be a DER or PEM file",
                );
            }
        };
    }

    for proxy in &options.proxies {
        trace!(proxy, "Adding proxy to http client");

        if proxy.starts_with("http:") {
            client = client.proxy(reqwest::Proxy::http(proxy).into_diagnostic()?);
        } else if proxy.starts_with("https:") {
            client = client.proxy(reqwest::Proxy::https(proxy).into_diagnostic()?);
        } else {
            warn!(proxy, "Invalid proxy, only http or https URLs allowed");
        };
    }

    let client = client.build().into_diagnostic()?;

    debug!("Created HTTP client");

    Ok(client)
}
