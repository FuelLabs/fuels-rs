use fuels::types::errors::Result;

use crate::aws_kms::{AwsKms, AwsKmsProcess};

pub async fn start_aws_kms(logs: bool) -> Result<AwsKmsProcess> {
    AwsKms::default().with_show_logs(logs).start().await
}
