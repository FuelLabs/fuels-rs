use aws_config::{
    default_provider::credentials::DefaultCredentialsChain, BehaviorVersion, Region, SdkConfig,
};
use aws_sdk_kms::Client;

#[derive(Clone)]
pub struct AwsConfig {
    sdk_config: SdkConfig,
}

impl AwsConfig {
    pub async fn from_environment() -> Self {
        let loader = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(DefaultCredentialsChain::builder().build().await);

        Self {
            sdk_config: loader.load().await,
        }
    }

    #[cfg(feature = "test-helpers")]
    pub async fn for_testing(endpoint_url: String) -> Self {
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(aws_sdk_kms::config::Credentials::new(
                "test",
                "test",
                None,
                None,
                "Static Test Credentials",
            ))
            .endpoint_url(endpoint_url)
            .region(Region::new("us-east-1")) // Test region
            .load()
            .await;

        Self { sdk_config }
    }

    pub fn endpoint_url(&self) -> Option<&str> {
        self.sdk_config.endpoint_url()
    }

    pub fn region(&self) -> Option<&Region> {
        self.sdk_config.region()
    }
}

#[derive(Clone, Debug)]
pub struct AwsClient {
    client: Client,
}

impl AwsClient {
    pub fn new(config: AwsConfig) -> Self {
        let config = config.sdk_config;
        let client = Client::new(&config);

        Self { client }
    }

    pub fn inner(&self) -> &aws_sdk_kms::Client {
        &self.client
    }
}
