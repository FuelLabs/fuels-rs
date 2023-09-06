use fuels_core::types::errors::Result;
use std::fmt::Debug;
use std::future::Future;
use std::num::NonZeroUsize;
use std::time::Duration;

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
    pub fn wait_duration(&self, attempt: u32) -> Duration {
        match self {
            Backoff::Linear(base_duration) => *base_duration * (attempt + 1),
            Backoff::Exponential(base_duration) => *base_duration * 2u32.pow(attempt),
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
/// use std::num::NonZeroUsize;
/// use std::time::Duration;
/// use fuels_programs::retry::{Backoff, RetryConfig};
///
/// let max_attempts = NonZeroUsize::new(5).unwrap();
/// let interval_strategy = Backoff::Exponential(Duration::from_secs(1));
///
/// let retry_config = RetryConfig::new(max_attempts, interval_strategy);
/// ```
#[derive(Clone, Debug)]
pub struct RetryConfig {
    pub max_attempts: NonZeroUsize,
    pub interval: Backoff,
}

impl RetryConfig {
    pub fn new(max_attempts: NonZeroUsize, interval: Backoff) -> Self {
        RetryConfig {
            max_attempts,
            interval,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: NonZeroUsize::new(1).expect("Should not fail!"),
            interval: Default::default(),
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
/// Returns `Err(Error)` if the maximum number of attempts is reached and the action
/// still fails. If a retryable error occurs during the attempts, the error will
/// be returned if the `should_retry` condition allows further retries.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use fuels_core::types::errors::Error;
/// use std::num::NonZeroUsize;
/// use fuels_programs::retry::{Backoff, retry, RetryConfig};
///
/// async fn network_request() -> Result<(), Error> {
///     // Simulate network request here
///     // ...
///     // For demonstration purposes, always return an error
///   Err(Error::InvalidData("Error".to_string()))
/// }
///
/// fn main() {
///
///     let retry_config = RetryConfig {
///         max_attempts: NonZeroUsize::new(3).unwrap(),
///         interval: Backoff::Linear(Duration::from_secs(1)),
///     };
///
///     let should_retry = |result: &Result<(), Error>| {
///         // Retry if the error is retryable
///         result.is_err()
///     };
///
///     let result = retry(network_request, &retry_config, should_retry);
/// }
/// ```
pub async fn retry<Fut, T, ShouldRetry>(
    mut action: impl FnMut() -> Fut,
    retry_config: &RetryConfig,
    should_retry: ShouldRetry,
) -> Result<T>
where
    Fut: Future<Output = Result<T>>,
    ShouldRetry: Fn(&Result<T>) -> bool,
{
    let mut last_result = None;

    for attempt in 0..retry_config.max_attempts.into() {
        let result = action().await;

        if should_retry(&result) {
            last_result = Some(result)
        } else {
            return result;
        }

        tokio::time::sleep(retry_config.interval.wait_duration(attempt)).await;
    }

    last_result.expect("Should not happen")
}

#[cfg(test)]
mod tests {
    mod retry_until {
        use crate::retry::{retry, Backoff, RetryConfig};
        use fuel_tx::TxId;
        use fuels_core::types::errors::Error;
        use std::num::NonZeroUsize;
        use std::str::FromStr;
        use std::time::{Duration, Instant};
        use tokio::sync::Mutex;

        #[tokio::test]
        async fn returns_last_error() -> anyhow::Result<()> {
            let err_msgs = ["Err1", "Err2", "Err3"];
            let number_of_attempts = Mutex::new(0usize);

            let will_always_fail = || async {
                let msg = err_msgs[*number_of_attempts.lock().await];
                *number_of_attempts.lock().await += 1;

                Result::<(), _>::Err(Error::InvalidData(msg.to_string()))
            };

            let should_retry_fn = |_res: &_| -> bool { true };

            let max_attempts = NonZeroUsize::new(3).unwrap();

            let retry_options =
                RetryConfig::new(max_attempts, Backoff::Linear(Duration::from_millis(10)));

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
            let max_attempts = NonZeroUsize::new(5).unwrap();

            let retry_options =
                RetryConfig::new(max_attempts, Backoff::Linear(Duration::from_millis(10)));

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
            let max_attempts = NonZeroUsize::new(5).unwrap();

            let retry_options =
                RetryConfig::new(max_attempts, Backoff::Linear(Duration::from_millis(10)));

            let ok = retry(will_always_fail, &retry_options, should_retry_fn).await?;

            assert_eq!(ok.unwrap(), "Success");

            Ok(())
        }

        #[tokio::test]
        async fn return_on_last_attempt() -> anyhow::Result<()> {
            let values = Mutex::new(vec![Ok::<Option<String>, Error>(None), Ok(None), Ok(None)]);
            let will_always_fail = || async { values.lock().await.pop().unwrap() };

            let should_retry_fn = |res: &_| -> bool {
                match res {
                    Err(err) if matches!(err, Error::IOError(_)) => true,
                    Ok(None) => true,
                    _ => false,
                }
            };
            let max_attempts = NonZeroUsize::new(3).unwrap();

            let retry_options =
                RetryConfig::new(max_attempts, Backoff::Linear(Duration::from_millis(10)));

            let ok = retry(will_always_fail, &retry_options, should_retry_fn).await?;

            dbg!(&ok);

            assert_eq!(ok, None);

            Ok(())
        }

        #[tokio::test]
        async fn retry_on_io_error() -> anyhow::Result<()> {
            let tx_id = TxId::from_str(
                "0x98f01c73c2062b55bba70966917a0839995e86abfadfff24534262d1c8b7a64e",
            );
            let values = Mutex::new(vec![
                Ok(tx_id),
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

            let max_attempts = NonZeroUsize::new(3).unwrap();

            let retry_options =
                RetryConfig::new(max_attempts, Backoff::Linear(Duration::from_millis(10)));

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

            let max_attempts = NonZeroUsize::new(3).unwrap();

            let retry_options =
                RetryConfig::new(max_attempts, Backoff::Fixed(Duration::from_millis(100)));

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

            let max_attempts = NonZeroUsize::new(3).unwrap();

            let retry_options =
                RetryConfig::new(max_attempts, Backoff::Linear(Duration::from_millis(100)));

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
                .enumerate()
                .all(|(attempt, (current_timestamp, the_next_timestamp))| {
                    the_next_timestamp.duration_since(*current_timestamp)
                        >= (Duration::from_millis(100) * (attempt + 1) as u32)
                });

            assert!(
                timestamps_spaced_out_at_least_100_mills,
                "Retry did not wait for the specified time between attempts."
            );

            Ok(())
        }

        #[tokio::test]
        async fn retry_respects_delay_between_attempts_exponential() -> anyhow::Result<()> {
            let timestamps: Mutex<Vec<Instant>> = Mutex::new(vec![]);

            let will_fail_and_record_timestamp = || async {
                timestamps.lock().await.push(Instant::now());
                Result::<(), _>::Err(Error::InvalidData("Error".to_string()))
            };

            let should_retry_fn = |_res: &_| -> bool { true };
            let max_attempts = NonZeroUsize::new(3).unwrap();

            let retry_options = RetryConfig::new(
                max_attempts,
                Backoff::Exponential(Duration::from_millis(100)),
            );

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
                .enumerate()
                .all(|(attempt, (current_timestamp, the_next_timestamp))| {
                    the_next_timestamp.duration_since(*current_timestamp)
                        >= (Duration::from_millis(100) * (2_usize.pow((attempt) as u32)) as u32)
                });

            assert!(
                timestamps_spaced_out_at_least_100_mills,
                "Retry did not wait for the specified time between attempts."
            );

            Ok(())
        }
    }
}
