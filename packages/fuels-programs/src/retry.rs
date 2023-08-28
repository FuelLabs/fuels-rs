use std::error::Error;
use std::future::Future;
use std::num::NonZeroUsize;
use std::time::Duration;

use std::fmt;
use std::fmt::Debug;
use std::sync::Arc;

type RetryOn = Option<Arc<dyn Fn(&dyn Error) -> bool + Send + Sync>>;

#[derive(Clone)]
pub struct RetryConfig {
    pub max_attempts: NonZeroUsize,
    pub interval: Duration,
    retry_on: RetryOn,
    retry_on_none: bool,
}

impl fmt::Debug for RetryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RetryOptions {{ max_attempts: {:?}, interval: {:?}, retry_on_errors: {:?} }}",
            self.max_attempts,
            self.interval,
            self.retry_on.as_ref().map(|_| "Some(...)")
        )
    }
}

impl RetryConfig {
    pub fn new(max_attempts: NonZeroUsize, interval: Duration) -> Self {
        RetryConfig {
            max_attempts,
            interval,
            retry_on: None,
            retry_on_none: false,
        }
    }

    pub fn set_retry_on<F>(&mut self, retry_on: F) -> RetryConfig
    where
        F: Fn(&dyn Error) -> bool + Send + Sync + 'static,
    {
        self.retry_on = Some(Arc::new(retry_on));
        self.clone()
    }

    pub fn retry_on_none(mut self) -> RetryConfig {
        self.retry_on_none = true;
        self
    }
}

pub async fn retry<Fut, T, K, ShouldRetry>(
    mut action: impl FnMut() -> Fut,
    retry_options: &RetryConfig,
    should_retry: ShouldRetry,
) -> Result<T, K>
where
    T: Clone + Debug,
    Fut: Future<Output = Result<T, K>>,
    K: Clone + Error + 'static,
    ShouldRetry: Fn(&Result<T, K>) -> bool,
{
    let mut last_err = None;
    let max_attempts = retry_options.max_attempts.get();

    for _ in 0..max_attempts {
        let result = action().await;

        match result.clone() {
            Ok(value) => {
                if !should_retry(&result) {
                    return Ok(value);
                }
            }
            Err(error) => {
                if should_retry(&result) {
                    last_err = Some(error);
                } else {
                    return Err(error);
                }
            }
        }

        tokio::time::sleep(retry_options.interval).await;
    }

    Err(last_err.expect("Retry Must have failed"))
}

#[cfg(test)]
mod tests {
    mod retry_until {
        use crate::retry::{retry, RetryConfig};
        use fuels_core::types::errors::Error;
        use std::time::{Duration, Instant};
        use tokio::sync::Mutex;

        #[tokio::test]
        async fn gives_up_after_max_attempts() -> anyhow::Result<()> {
            let number_of_attempts = Mutex::new(0usize);

            let will_always_fail = || async {
                *number_of_attempts.lock().await += 1;

                Result::<(), _>::Err(Error::InvalidData("Error".to_string()))
            };

            let should_retry_fn = |_res: &_| -> bool { true };

            let retry_options = RetryConfig::new(3.try_into().unwrap(), Duration::from_millis(10));

            let _ = retry(will_always_fail, &retry_options, should_retry_fn).await;

            assert_eq!(*number_of_attempts.lock().await, 3);

            Ok(())
        }

        #[tokio::test]
        async fn returns_last_error() -> fuels_core::types::errors::Result<()> {
            let err_msgs = ["Err1", "Err2", "Err3"];
            let number_of_attempts = Mutex::new(0usize);

            let will_always_fail = || async {
                let msg = err_msgs[*number_of_attempts.lock().await];
                *number_of_attempts.lock().await += 1;

                Result::<(), _>::Err(Error::InvalidData(msg.to_string()))
            };

            let should_retry_fn = |_res: &_| -> bool { true };

            let retry_options = RetryConfig::new(3.try_into().unwrap(), Duration::from_millis(10));

            let err = retry(will_always_fail, &retry_options, should_retry_fn)
                .await
                .expect_err("Should have failed");

            assert_eq!(
                err.to_string(),
                Error::InvalidData(err_msgs[2].to_string()).to_string()
            );

            Ok(())
        }

        #[tokio::test]
        async fn returns_value_on_success() -> anyhow::Result<()> {
            let values = Mutex::new(vec![
                Ok(String::from("Success")),
                Err(Error::InvalidData("Err1".to_string())),
                Err(Error::InvalidData("Err2".to_string())),
            ]);

            let will_always_fail = || async { values.lock().await.pop().unwrap() };

            let should_retry_fn = |res: &_| -> bool {
                matches!(res, Err(err) if matches!(err, Error::InvalidData(_)))
            };

            let retry_options = RetryConfig::new(3.try_into().unwrap(), Duration::from_millis(10));

            let ok = retry(will_always_fail, &retry_options, should_retry_fn).await?;

            assert_eq!(ok, "Success");

            Ok(())
        }

        #[tokio::test]
        async fn retry_on_none_values() -> anyhow::Result<()> {
            let values = Mutex::new(vec![
                Ok::<Option<String>, Error>(Some(String::from("Success"))),
                Ok(None),
                Ok(None),
            ]);
            let will_always_fail = || async { values.lock().await.pop().unwrap() };

            let should_retry_fn = |res: &_| -> bool {
                match res {
                    Err(err) if matches!(err, Error::IOError(_)) => true,
                    Ok(None) => true,
                    _ => false,
                }
            };

            let retry_options = RetryConfig::new(3.try_into().unwrap(), Duration::from_millis(10));

            let ok = retry(will_always_fail, &retry_options, should_retry_fn).await?;

            assert_eq!(ok.unwrap(), "Success");

            Ok(())
        }

        #[tokio::test]
        async fn retry_respects_delay_between_attempts() -> anyhow::Result<()> {
            let timestamps: Mutex<Vec<Instant>> = Mutex::new(vec![]);

            let will_fail_and_record_timestamp = || async {
                timestamps.lock().await.push(Instant::now());
                Result::<(), _>::Err(Error::InvalidData("Error".to_string()))
            };

            let should_retry_fn = |_res: &_| -> bool { true };

            let retry_options = RetryConfig::new(3.try_into().unwrap(), Duration::from_millis(100));

            let _ = retry(
                will_fail_and_record_timestamp,
                &retry_options,
                should_retry_fn,
            )
            .await;

            let timestamps_vec = timestamps.lock().await.clone();

            let timestamps_spaced_out_at_least_100_mills = timestamps_vec
                .iter()
                .zip(timestamps_vec.iter().skip(1))
                .all(|(current_timestamp, the_next_timestamp)| {
                    *the_next_timestamp - *current_timestamp >= Duration::from_millis(100)
                });

            assert!(
                timestamps_spaced_out_at_least_100_mills,
                "Retry did not wait for the specified time between attempts."
            );

            Ok(())
        }
    }
}
