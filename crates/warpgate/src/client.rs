use crate::error::WarpgateError;
use async_trait::async_trait;
use core::ops::Deref;
use miette::IntoDiagnostic;
use netrc::Netrc;
use reqwest::{Client, Response, Url};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, RequestBuilder, RequestInitialiser};
use serde::{Deserialize, Serialize};
use starbase_utils::{
    fs,
    net::{Downloader, NetError},
};
use std::path::PathBuf;
use tracing::{debug, trace, warn};

pub struct HttpDownloader {
    client: HttpClient,
}

#[async_trait]
impl Downloader for HttpDownloader {
    async fn download(&self, url: Url) -> Result<Response, NetError> {
        let url_string = url.to_string();

        self.client
            .get(url)
            .send()
            .await
            .map_err(|error| match error {
                reqwest_middleware::Error::Middleware(inner) => NetError::HttpUnknown {
                    error: format!("{}", inner),
                    url: url_string,
                },
                reqwest_middleware::Error::Reqwest(inner) => NetError::Http {
                    error: Box::new(inner),
                    url: url_string,
                },
            })
    }
}

// `ClientWithMiddleware` doesn't allow access to their inner `Client`,
// so we unfortunately need to keep a reference to both.
// https://github.com/TrueLayer/reqwest-middleware/issues/203
#[derive(Clone, Default)]
pub struct HttpClient {
    client: Client,
    middleware: ClientWithMiddleware,
}

impl HttpClient {
    pub fn create_downloader(&self) -> HttpDownloader {
        HttpDownloader {
            client: self.clone(),
        }
    }

    pub fn to_inner(&self) -> &Client {
        &self.client
    }

    pub fn map_error(url: String, error: reqwest_middleware::Error) -> WarpgateError {
        match error {
            reqwest_middleware::Error::Middleware(inner) => WarpgateError::HttpMiddleware {
                error: format!("{}", inner),
                url,
            },
            reqwest_middleware::Error::Reqwest(inner) => WarpgateError::Http {
                error: Box::new(inner),
                url,
            },
        }
    }
}

impl Deref for HttpClient {
    type Target = ClientWithMiddleware;

    fn deref(&self) -> &Self::Target {
        &self.middleware
    }
}

/// Configures the HTTPS client used for making requests.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
pub struct HttpOptions {
    /// Allow invalid certificates. This is dangerous and should only be used as a last resort!
    pub allow_invalid_certs: bool,

    /// Absolute path to a directory in which to cache GET and HEAD requests.
    pub cache_dir: Option<PathBuf>,

    /// A list of proxy URLs that all requests should pass through. URLs that start with
    /// `http:` will handle insecure requests, while `https:` will handle secure requests.
    pub proxies: Vec<String>,

    /// A list of proxy URLs that all `https:` requests should pass through.
    pub secure_proxies: Vec<String>,

    /// Absolute path to the root certificate. Supports `.pem` and `.der` files.
    pub root_cert: Option<PathBuf>,
}

/// Create an HTTP/HTTPS client that'll be used for downloading files.
pub fn create_http_client() -> miette::Result<HttpClient> {
    create_http_client_with_options(&HttpOptions::default())
}

/// Create an HTTP/HTTPS client with the provided options, that'll be
/// used for downloading files.
pub fn create_http_client_with_options(options: &HttpOptions) -> miette::Result<HttpClient> {
    debug!("Creating HTTP client");

    let mut client_builder = reqwest::Client::builder()
        .user_agent(format!("warpgate@{}", env!("CARGO_PKG_VERSION")))
        .use_rustls_tls();

    if options.allow_invalid_certs {
        trace!("Allowing invalid certificates (I hope you know what you're doing!)");

        client_builder = client_builder.danger_accept_invalid_certs(true);
    }

    if let Some(root_cert) = &options.root_cert {
        trace!(root_cert = ?root_cert, "Adding user provided root certificate");

        match root_cert.extension().and_then(|ext| ext.to_str()) {
            Some("der") => {
                client_builder = client_builder.add_root_certificate(
                    reqwest::Certificate::from_der(&fs::read_file_bytes(root_cert)?)
                        .into_diagnostic()?,
                )
            }
            Some("pem") => {
                client_builder = client_builder.add_root_certificate(
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
            client_builder = client_builder.proxy(reqwest::Proxy::http(proxy).into_diagnostic()?);
        }
    }

    if !secure_proxies.is_empty() {
        trace!(proxies = ?secure_proxies, "Adding secure proxies to client");

        for proxy in secure_proxies {
            client_builder = client_builder.proxy(reqwest::Proxy::https(proxy).into_diagnostic()?);
        }
    }

    let client = client_builder.build().into_diagnostic()?;

    trace!("Applying middleware to client");

    let mut middleware_builder = ClientBuilder::new(client.clone());

    if let Ok(netrc) = NetrcMiddleware::new() {
        trace!("Adding .netrc support");

        middleware_builder = middleware_builder.with_init(netrc);
    }

    if let Some(cache_dir) = &options.cache_dir {
        use http_cache_reqwest::{
            CACacheManager, Cache, CacheMode, CacheOptions, HttpCache, HttpCacheOptions,
        };

        trace!("Adding GET and HEAD request caching");

        middleware_builder = middleware_builder.with(Cache(HttpCache {
            manager: CACacheManager {
                path: cache_dir.to_owned(),
            },
            mode: CacheMode::Default,
            options: HttpCacheOptions {
                // https://github.com/kornelski/rusty-http-cache-semantics
                cache_options: Some(CacheOptions {
                    cache_heuristic: 0.025,
                    ..Default::default()
                }),
                ..Default::default()
            },
        }));
    }

    let middleware = middleware_builder.build();

    debug!("Created HTTP client");

    Ok(HttpClient { client, middleware })
}

// TODO: Temporary until this lands
// https://github.com/gribouille/netrc/issues/9
pub struct NetrcMiddleware {
    nrc: Netrc,
}

impl NetrcMiddleware {
    pub fn new() -> netrc::Result<Self> {
        Netrc::new().map(|nrc| NetrcMiddleware { nrc })
    }
}

impl RequestInitialiser for NetrcMiddleware {
    fn init(&self, req: RequestBuilder) -> RequestBuilder {
        match req.try_clone() {
            Some(nr) => req
                .try_clone()
                .unwrap()
                .build()
                .ok()
                .and_then(|r| {
                    r.url()
                        .host_str()
                        .and_then(|host| {
                            self.nrc
                                .hosts
                                .get(host)
                                .or_else(|| self.nrc.hosts.get("default"))
                        })
                        .map(|auth| {
                            nr.basic_auth(
                                &auth.login,
                                if auth.password.is_empty() {
                                    None
                                } else {
                                    Some(&auth.password)
                                },
                            )
                        })
                })
                .unwrap_or(req),
            None => req,
        }
    }
}
