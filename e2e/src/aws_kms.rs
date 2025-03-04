use fuels::accounts::kms::{
    aws_config::{defaults, BehaviorVersion, Region},
    aws_sdk_kms::{
        config::Credentials,
        types::{KeySpec, KeyUsageType},
        Client,
    },
    KmsKey,
};
use fuels::prelude::Error;
use fuels::types::errors::Context;
use fuels::types::errors::Result;
use testcontainers::{core::ContainerPort, runners::AsyncRunner};
use tokio::io::AsyncBufReadExt;

#[derive(Default)]
pub struct AwsKms {
    show_logs: bool,
}

struct AwsKmsImage;

impl testcontainers::Image for AwsKmsImage {
    fn name(&self) -> &str {
        "localstack/localstack"
    }

    fn tag(&self) -> &str {
        "latest"
    }

    fn ready_conditions(&self) -> Vec<testcontainers::core::WaitFor> {
        vec![testcontainers::core::WaitFor::message_on_stdout("Ready.")]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[ContainerPort::Tcp(4566)]
    }
}

impl AwsKms {
    pub fn with_show_logs(mut self, show_logs: bool) -> Self {
        self.show_logs = show_logs;
        self
    }

    pub async fn start(self) -> Result<AwsKmsProcess> {
        let container = AwsKmsImage
            .start()
            .await
            .map_err(|e| Error::Other(e.to_string()))
            .with_context(|| "Failed to start KMS container")?;

        if self.show_logs {
            spawn_log_printer(&container);
        }

        let port = container
            .get_host_port_ipv4(4566)
            .await
            .map_err(|e| Error::Other(e.to_string()))?;
        let url = format!("http://localhost:{}", port);

        let credentials = Credentials::new("test", "test", None, None, "Static Test Credentials");
        let region = Region::new("us-east-1");

        let config = defaults(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .endpoint_url(url.clone())
            .region(region)
            .load()
            .await;

        let client = Client::new(&config);

        Ok(AwsKmsProcess {
            _container: container,
            client,
            url,
        })
    }
}

fn spawn_log_printer(container: &testcontainers::ContainerAsync<AwsKmsImage>) {
    let stderr = container.stderr(true);
    let stdout = container.stdout(true);
    tokio::spawn(async move {
        let mut stderr_lines = stderr.lines();
        let mut stdout_lines = stdout.lines();

        let mut other_stream_closed = false;
        loop {
            tokio::select! {
                stderr_result = stderr_lines.next_line() => {
                    match stderr_result {
                        Ok(Some(line)) => eprintln!("KMS (stderr): {}", line),
                        Ok(None) if other_stream_closed => break,
                        Ok(None) => other_stream_closed = true,
                        Err(e) => {
                            eprintln!("KMS: Error reading from stderr: {:?}", e);
                            break;
                        }
                    }
                }
                stdout_result = stdout_lines.next_line() => {
                    match stdout_result {
                        Ok(Some(line)) => eprintln!("KMS (stdout): {}", line),
                        Ok(None) if other_stream_closed => break,
                        Ok(None) => other_stream_closed = true,
                        Err(e) => {
                            eprintln!("KMS: Error reading from stdout: {:?}", e);
                            break;
                        }
                    }
                }
            }
        }

        Ok::<(), std::io::Error>(())
    });
}

pub struct AwsKmsProcess {
    _container: testcontainers::ContainerAsync<AwsKmsImage>,
    client: Client,
    url: String,
}

impl AwsKmsProcess {
    pub async fn create_key(&self) -> anyhow::Result<KmsTestKey> {
        let response = self
            .client
            .create_key()
            .key_usage(KeyUsageType::SignVerify)
            .key_spec(KeySpec::EccSecgP256K1)
            .send()
            .await?;

        let id = response
            .key_metadata
            .and_then(|metadata| metadata.arn)
            .ok_or_else(|| anyhow::anyhow!("key arn missing from response"))?;

        let kms_key = KmsKey::new(id.clone(), &self.client).await?;

        Ok(KmsTestKey {
            id,
            kms_key,
            url: self.url.clone(),
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

#[derive(Debug, Clone)]
pub struct KmsTestKey {
    pub id: String,
    pub kms_key: KmsKey,
    pub url: String,
}
