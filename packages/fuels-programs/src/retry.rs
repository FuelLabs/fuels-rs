use std::error::Error;
use std::future::Future;
use std::time::Duration;

use std::fmt::Debug;

/// A set of strategies to control retry intervals between attempts.
///
/// The `Backoff` enum defines different strategies for managing intervals between retry attempts.
/// Each strategy allows you to customize the waiting time before a new attempt based on the
/// number of attempts made.
///
/// # Variants
///
/// - `Linear(Duration)`: Increases the waiting time linearly with each attempt.
/// - `Exponential(Duration)`: Doubles the waiting time with each attempt.
/// - `Fixed(Duration)`: Uses a constant waiting time between attempts.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use fuels_programs::retry::Backoff;
///
/// let linear_backoff = Backoff::Linear(Duration::from_secs(2));
/// let exponential_backoff = Backoff::Exponential(Duration::from_secs(1));
/// let fixed_backoff = Backoff::Fixed(Duration::from_secs(5));
/// ```
#[derive(Debug, Clone)]
pub enum Backoff {
    Linear(Duration),
    Exponential(Duration),
    Fixed(Duration),
}

impl Default for Backoff {
    fn default() -> Self {
        Backoff::Linear(Duration::from_millis(10))
    }
}

impl Backoff {
    pub fn wait_duration(&self, attempt: usize) -> Duration {
        match self {
            Backoff::Linear(base_duration) => *base_duration * (attempt) as u32,
            Backoff::Exponential(base_duration) => {
                *base_duration * (2_usize.pow((attempt) as u32)) as u32
            }
            Backoff::Fixed(interval) => *interval,
        }
    }
}

/// Configuration for controlling retry behavior.
///
/// The `RetryConfig` struct encapsulates the configuration parameters for controlling the retry behavior
/// of asynchronous actions. It includes the maximum number of attempts and the interval strategy from
/// the `Backoff` enum that determines how much time to wait between retry attempts.
///
/// # Fields
///
/// - `max_attempts`: The maximum number of attempts before giving up.
/// - `interval`: The chosen interval strategy from the `Backoff` enum.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use fuels_programs::retry::{Backoff, RetryConfig};
///
/// let max_attempts = 5;
/// let interval_strategy = Backoff::Exponential(Duration::from_secs(1));
///
/// let retry_config = RetryConfig::new(max_attempts, interval_strategy);
/// ```
#[derive(Clone, Debug, Default)]
pub struct RetryConfig {
    pub max_attempts: usize,
    pub interval: Backoff,
}

impl RetryConfig {
    pub fn new(max_attempts: usize, interval: Backoff) -> Self {
        RetryConfig {
            max_attempts,
            interval,
        }
    }
}

/// Retries an asynchronous action with customizable retry behavior.
///
/// This function takes an asynchronous action represented by a closure `action`.
/// The action is executed repeatedly with backoff and retry logic based on the
/// provided `retry_config` and the `should_retry` condition.
///
/// The `action` closure should return a `Future` that resolves to a `Result<T, K>`,
/// where `T` represents the success type and `K` represents the error type.
///
/// # Parameters
///
/// - `action`: The asynchronous action to be retried.
/// - `retry_config`: A reference to the retry configuration.
/// - `should_retry`: A closure that determines whether to retry based on the result.
///
/// # Return
///
/// Returns `Ok(T)` if the action succeeds without requiring further retries.
/// Returns `Err(K)` if the maximum number of attempts is reached and the action
/// still fails. If a retryable error occurs during the attempts, the error will
/// be returned if the `should_retry` condition allows further retries.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use tokio::time::sleep;
/// use std::error::Error;
/// use thiserror::Error;
///
/// use fuels_programs::retry::{Backoff, retry, RetryConfig};
///
/// #[derive(Debug, Error, Clone)]
/// pub enum MyError {
///     #[error("Network error: {0}")]
///     NetworkError(String),
/// }
///
/// async fn network_request() -> Result<(), MyError> {
///     // Simulate network request here
///     // ...
///     // For demonstration purposes, always return an error
///   Err(MyError::NetworkError("Network error".into()))
/// }
///
/// fn main() {
///     let retry_config = RetryConfig {
///         max_attempts: 3,
///         interval: Backoff::Linear(Duration::from_secs(1)),
///     };
///
///     let should_retry = |result: &Result<(), MyError>| {
///         // Retry if the error is retryable
///         result.is_err()
///     };
///
///     let result = retry(network_request, &retry_config, should_retry);
/// }
/// ```
pub async fn retry<Fut, T, K, ShouldRetry>(
    mut action: impl FnMut() -> Fut,
    retry_config: &RetryConfig,
    should_retry: ShouldRetry,
) -> Result<T, K>
where
    T: Clone + Debug,
    Fut: Future<Output = Result<T, K>>,
    K: Clone + Error + 'static,
    ShouldRetry: Fn(&Result<T, K>) -> bool,
{
    let mut last_err = None;
    let max_attempts = retry_config.max_attempts;

    for attempt in 1..max_attempts + 1 {
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

        tokio::time::sleep(retry_config.interval.wait_duration(attempt)).await;
    }

    Err(last_err.expect("Retry must have failed"))
}

#[cfg(test)]
mod tests {
    mod retry_until {
        use crate::retry::{retry, Backoff, RetryConfig};
        use fuel_tx::TxId;
        use fuels_core::types::errors::Error;
        use std::str::FromStr;
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

            let retry_options = RetryConfig::new(3, Backoff::Linear(Duration::from_millis(10)));

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

            let retry_options = RetryConfig::new(3, Backoff::Linear(Duration::from_millis(10)));

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

            let retry_options = RetryConfig::new(3, Backoff::Linear(Duration::from_millis(10)));

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

            let retry_options = RetryConfig::new(3, Backoff::Linear(Duration::from_millis(10)));

            let ok = retry(will_always_fail, &retry_options, should_retry_fn).await?;

            assert_eq!(ok.unwrap(), "Success");

            Ok(())
        }

        #[tokio::test]
        async fn retry_on_io_error() -> anyhow::Result<()> {
            let values = Mutex::new(vec![
                Ok(TxId::from_str(
                    "0x98f01c73c2062b55bba70966917a0839995e86abfadfff24534262d1c8b7a64e",
                )),
                Err(Error::IOError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed".to_string(),
                ))),
                Err(Error::IOError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed".to_string(),
                ))),
            ]);
            let will_always_fail = || async { values.lock().await.pop().unwrap() };

            let should_retry_fn = |res: &_| -> bool { matches!(res, Err(Error::IOError(_))) };

            let retry_options = RetryConfig::new(3, Backoff::Linear(Duration::from_millis(10)));

            let ok = retry(will_always_fail, &retry_options, should_retry_fn).await?;

            assert_eq!(
                ok,
                TxId::from_str(
                    "0x98f01c73c2062b55bba70966917a0839995e86abfadfff24534262d1c8b7a64e"
                )
            );

            Ok(())
        }

        #[tokio::test]
        async fn retry_respects_delay_between_attempts_fixed() -> anyhow::Result<()> {
            let timestamps: Mutex<Vec<Instant>> = Mutex::new(vec![]);

            let will_fail_and_record_timestamp = || async {
                timestamps.lock().await.push(Instant::now());
                Result::<(), _>::Err(Error::InvalidData("Error".to_string()))
            };

            let should_retry_fn = |_res: &_| -> bool { true };

            let retry_options = RetryConfig::new(3, Backoff::Fixed(Duration::from_millis(100)));

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
                    the_next_timestamp.duration_since(*current_timestamp)
                        >= Duration::from_millis(100)
                });

            assert!(
                timestamps_spaced_out_at_least_100_mills,
                "Retry did not wait for the specified time between attempts."
            );

            Ok(())
        }

        #[tokio::test]
        async fn retry_respects_delay_between_attempts_linear() -> anyhow::Result<()> {
            let timestamps: Mutex<Vec<Instant>> = Mutex::new(vec![]);

            let will_fail_and_record_timestamp = || async {
                timestamps.lock().await.push(Instant::now());
                Result::<(), _>::Err(Error::InvalidData("Error".to_string()))
            };

            let should_retry_fn = |_res: &_| -> bool { true };

            let retry_options = RetryConfig::new(2, Backoff::Linear(Duration::from_millis(100)));

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
                    the_next_timestamp.duration_since(*current_timestamp)
                        >= Duration::from_millis(100)
                });

            assert!(
                timestamps_spaced_out_at_least_100_mills,
                "Retry did not wait for the specified time between attempts."
            );

            Ok(())
        }
    }
}