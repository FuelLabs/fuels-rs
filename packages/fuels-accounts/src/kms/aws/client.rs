pub use aws_config::{
    default_provider::credentials::DefaultCredentialsChain, defaults, BehaviorVersion, Region,
    SdkConfig,
};
pub use aws_sdk_kms::config::Credentials;
use aws_sdk_kms::Client;

#[derive(Clone, Debug)]
pub struct AwsClient {
    client: Client,
}

impl AwsClient {
    pub fn new(config: SdkConfig) -> Self {
        let client = Client::new(&config);

        Self { client }
    }

    pub fn inner(&self) -> &Client {
        &self.client
    }
}
