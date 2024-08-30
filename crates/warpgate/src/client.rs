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

    /// A list of proxy URLs that all requests should pass through. URLS that start with
    /// `http` will handle insecure requests, while `https` will handle secure requests.
    pub proxies: Vec<String>,

    /// A list of proxy URLs that all `https` requests should pass through.
    pub secure_proxies: Vec<String>,

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

    let mut insecure_proxies = vec![];
    let mut secure_proxies = options.secure_proxies.iter().collect::<Vec<_>>();

    for proxy in &options.proxies {
        if proxy.starts_with("https:") || (proxy.starts_with("http:") && proxy.contains(":443")) {
            secure_proxies.push(proxy);
        } else if proxy.starts_with("http:") {
            insecure_proxies.push(proxy);
        } else {
            warn!(proxy, "Invalid proxy, only http or https URLs allowed");
        };
    }

    if !insecure_proxies.is_empty() {
        trace!(proxies = ?insecure_proxies, "Adding insecure proxies to client");

        for proxy in insecure_proxies {
            client = client.proxy(reqwest::Proxy::http(proxy).into_diagnostic()?);
        }
    }

    if !secure_proxies.is_empty() {
        trace!(proxies = ?secure_proxies, "Adding secure proxies to client");

        for proxy in secure_proxies {
            client = client.proxy(reqwest::Proxy::https(proxy).into_diagnostic()?);
        }
    }

    let client = client.build().into_diagnostic()?;

    debug!("Created HTTP client");

    Ok(client)
}
