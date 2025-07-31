use oci_client::Client;
use std::ops::Deref;

/// An OCI client that wraps [`oci_client::Client`].
#[derive(Clone, Default)]
pub struct OciClient(Client);

impl Deref for OciClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
