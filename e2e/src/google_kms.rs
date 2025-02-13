use anyhow::Context;
use google_cloud_kms::grpc::kms::v1::crypto_key::CryptoKeyPurpose;
use google_cloud_kms::grpc::kms::v1::key_management_service_client::KeyManagementServiceClient;
use google_cloud_kms::grpc::kms::v1::CreateKeyRingRequest;
use google_cloud_kms::grpc::kms::v1::CryptoKey;
use google_cloud_kms::grpc::kms::v1::CryptoKeyVersionTemplate;
use google_cloud_kms::grpc::kms::v1::KeyRing;
use google_cloud_kms::grpc::kms::v1::ProtectionLevel;
use google_cloud_kms::grpc::kms::v1::{crypto_key_version, CreateCryptoKeyRequest};
use reqwest;
use serde_json::json;
use testcontainers::{core::ContainerPort, runners::AsyncRunner};
use tokio::io::AsyncBufReadExt;
use tonic::transport::Channel;

struct MockServerImage;

impl testcontainers::Image for MockServerImage {
    fn name(&self) -> &str {
        "mockserver/mockserver"
    }

    fn tag(&self) -> &str {
        "latest"
    }

    fn ready_conditions(&self) -> Vec<testcontainers::core::WaitFor> {
        vec![testcontainers::core::WaitFor::message_on_stdout(
            "started on port",
        )]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[ContainerPort::Tcp(1080)] // MockServer default port
    }
}

#[derive(Default)]
pub struct GoogleKms {
    show_logs: bool,
}

impl GoogleKms {
    pub fn with_show_logs(mut self, show_logs: bool) -> Self {
        self.show_logs = show_logs;
        self
    }

    pub async fn start(self) -> anyhow::Result<GoogleKmsProcess> {
        let container = MockServerImage
            .start()
            .await
            .with_context(|| "Failed to start MockServer container")?;

        if self.show_logs {
            spawn_log_printer(&container);
        }

        let port = container.get_host_port_ipv4(1080).await?;
        let mock_endpoint = format!("http://localhost:{}", port);

        // Configure mock responses
        Self::setup_mock_responses(&mock_endpoint).await?;

        // Create gRPC client with the mock endpoint
        let channel = Channel::from_shared(mock_endpoint.clone())
            .context("Invalid endpoint URL")?
            .connect()
            .await
            .context("Failed to connect to mock KMS")?;

        let client = KeyManagementServiceClient::new(channel);

        Ok(GoogleKmsProcess {
            _container: container,
            client,
            endpoint: mock_endpoint,
        })
    }

    async fn setup_mock_responses(endpoint: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();

        // Mock response for CreateKeyRing
        let create_keyring_expectation = json!({
            "httpRequest": {
                "method": "POST",
                "path": "/v1/projects/test-project/locations/global/keyRings.*"
            },
            "httpResponse": {
                "statusCode": 200,
                "headers": {
                    "content-type": ["application/json"]
                },
                "body": {
                    "name": "projects/test-project/locations/global/keyRings/fuel-test-keyring",
                    "createTime": "2024-02-13T10:00:00Z"
                }
            }
        });

        // Mock response for CreateCryptoKey
        let create_key_expectation = json!({
            "httpRequest": {
                "method": "POST",
                "path": "/v1/projects/test-project/locations/global/keyRings/.*/cryptoKeys.*"
            },
            "httpResponse": {
                "statusCode": 200,
                "headers": {
                    "content-type": ["application/json"]
                },
                "body": {
                    "name": "projects/test-project/locations/global/keyRings/fuel-test-keyring/cryptoKeys/fuel-signing-key",
                    "primary": {
                        "name": "projects/test-project/locations/global/keyRings/fuel-test-keyring/cryptoKeys/fuel-signing-key/cryptoKeyVersions/1",
                        "state": "ENABLED",
                        "algorithm": "EC_SIGN_SECP256K1_SHA256",
                        "protectionLevel": "SOFTWARE"
                    }
                }
            }
        });

        // Set up expectations
        client
            .put(format!("{}/mockserver/expectation", endpoint).as_str())
            .json(&create_keyring_expectation)
            .send()
            .await?;

        client
            .put(format!("{}/mockserver/expectation", endpoint).as_str())
            .json(&create_key_expectation)
            .send()
            .await?;

        Ok(())
    }
}

fn spawn_log_printer(container: &testcontainers::ContainerAsync<MockServerImage>) {
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
                        Ok(Some(line)) => eprintln!("MockServer (stderr): {}", line),
                        Ok(None) if other_stream_closed => break,
                        Ok(None) => other_stream_closed = true,
                        Err(e) => {
                            eprintln!("MockServer: Error reading from stderr: {:?}", e);
                            break;
                        }
                    }
                }
                stdout_result = stdout_lines.next_line() => {
                    match stdout_result {
                        Ok(Some(line)) => eprintln!("MockServer (stdout): {}", line),
                        Ok(None) if other_stream_closed => break,
                        Ok(None) => other_stream_closed = true,
                        Err(e) => {
                            eprintln!("MockServer: Error reading from stdout: {:?}", e);
                            break;
                        }
                    }
                }
            }
        }

        Ok::<(), std::io::Error>(())
    });
}

pub struct GoogleKmsProcess {
    _container: testcontainers::ContainerAsync<MockServerImage>,
    client: KeyManagementServiceClient<Channel>,
    pub endpoint: String,
}

impl GoogleKmsProcess {
    pub async fn create_key(&mut self) -> anyhow::Result<KmsTestKey> {
        // Create key ring
        let key_ring_id = "fuel-test-keyring";
        let parent = "projects/test-project/locations/global".to_string();
        let key_ring_path = format!("{}/keyRings/{}", parent, key_ring_id);

        let create_ring_result = self
            .client
            .create_key_ring(CreateKeyRingRequest {
                parent,
                key_ring_id: key_ring_id.to_string(),
                key_ring: Some(KeyRing::default()),
            })
            .await;

        if let Err(e) = &create_ring_result {
            if !e.to_string().contains("already exists") {
                create_ring_result.context("Failed to create key ring")?;
            }
        }

        // Create crypto key
        let key_id = "fuel-signing-key";
        let key_path = format!("{}/cryptoKeys/{}", key_ring_path, key_id);

        let request = CreateCryptoKeyRequest {
            parent: key_ring_path,
            crypto_key_id: key_id.to_string(),
            crypto_key: Some(CryptoKey {
                purpose: CryptoKeyPurpose::AsymmetricSign as i32,
                version_template: Some(CryptoKeyVersionTemplate {
                    algorithm: crypto_key_version::CryptoKeyVersionAlgorithm::EcSignP256Sha256
                        as i32,
                    protection_level: ProtectionLevel::Software as i32,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            skip_initial_version_creation: false,
        };

        let create_key_result = self.client.create_crypto_key(request).await;

        if let Err(e) = &create_key_result {
            if !e.to_string().contains("already exists") {
                create_key_result.context("Failed to create crypto key")?;
            }
        }

        Ok(KmsTestKey {
            name: key_path,
            endpoint: self.endpoint.clone(),
        })
    }
}
#[derive(Debug, Clone)]
pub struct KmsTestKey {
    pub name: String,
    pub endpoint: String,
}
