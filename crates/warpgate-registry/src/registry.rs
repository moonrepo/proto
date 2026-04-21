use crate::publish::PublishOptions;
use crate::registry_error::WarpgateRegistryError;
use warpgate::OciClient;

pub struct Registry {
    client: OciClient,
}

impl Registry {
    pub fn new(client: OciClient) -> Self {
        Self { client }
    }

    pub async fn publish(
        &self,
        _image: &str,
        _options: PublishOptions,
    ) -> Result<(), WarpgateRegistryError> {
        Ok(())
    }
}
