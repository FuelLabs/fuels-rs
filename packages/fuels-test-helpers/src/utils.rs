use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::time::Duration;

#[derive(Debug)]
pub struct RetryExhausted {
    interval: Duration,
    abort_after: Duration,
    error_from_last_attempt: Option<anyhow::Error>,
}

impl Display for RetryExhausted {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Attempted to retry action every {:?} for {:?}. The last attempt resulted in: {:?}",
            self.interval, self.abort_after, self.error_from_last_attempt
        )
    }
}

impl Error for RetryExhausted {}

pub async fn retry<Fut, T>(
    action: impl Fn() -> Fut,
    interval: Duration,
    abort_after: Duration,
) -> Result<T, RetryExhausted>
where
    Fut: Future<Output = anyhow::Result<T>>,
{
    let mut last_err = None;

    tokio::time::timeout(abort_after, async {
        loop {
            match action().await {
                Ok(value) => break value,
                Err(error) => last_err = Some(error),
            }

            tokio::time::sleep(interval).await;
        }
    })
    .await
    .map_err(|_| RetryExhausted {
        interval,
        abort_after,
        error_from_last_attempt: last_err,
    })
}

#[cfg(test)]
mod tests {
    mod retry_until {
        use crate::utils::retry;
        use anyhow::anyhow;
        use std::time::{Duration, Instant};
        use tokio::sync::Mutex;
        #[tokio::test]
        async fn gives_up_after_timeout() -> anyhow::Result<()> {
            let timestamp_of_last_attempt = Mutex::new(Instant::now());

            let will_always_fail = || async {
                *timestamp_of_last_attempt.lock().await = Instant::now();

                Ok(false)
            };

            let retry_start = Instant::now();
            retry(
                will_always_fail,
                Duration::from_millis(10),
                Duration::from_millis(250),
            )
            .await?;

            assert!(
                *timestamp_of_last_attempt.lock().await - retry_start < Duration::from_millis(250)
            );

            Ok(())
        }

        #[tokio::test]
        async fn returns_error_if_timeout_happened() -> anyhow::Result<()> {
            let will_always_fail =
                || async { Err(anyhow!("I fail because I must.")) as anyhow::Result<()> };

            let interval = Duration::from_millis(100);
            let abort_after = Duration::from_millis(250);

            let err = retry(will_always_fail, interval, abort_after)
                .await
                .expect_err("retry_until should have returned an error due to attempts exhaustion");

            assert_eq!(err.interval, interval);
            assert_eq!(err.abort_after, abort_after);
            assert!(err
                .error_from_last_attempt
                .expect("Must have the error since it ran at least once")
                .to_string()
                .contains("I fail because I must."));

            Ok(())
        }

        #[tokio::test]
        async fn returns_value_on_success() -> anyhow::Result<()> {
            let successfully_generates_value = || async { Ok(12345u64) as anyhow::Result<u64> };

            let value = retry(
                successfully_generates_value,
                Duration::from_millis(100),
                Duration::from_millis(250),
            )
            .await?;

            assert_eq!(value, 12345);

            Ok(())
        }

        #[tokio::test]
        async fn respects_delay_between_attempts() -> anyhow::Result<()> {
            let timestamps_predicate_was_called_at: Mutex<Vec<Instant>> = Mutex::new(vec![]);

            let will_fail = || async {
                timestamps_predicate_was_called_at
                    .lock()
                    .await
                    .push(Instant::now());
                Ok(false)
            };

            retry(
                will_fail,
                Duration::from_millis(100),
                Duration::from_millis(250),
            )
            .await?;

            let timestamps = timestamps_predicate_was_called_at.lock().await.clone();

            let timestamps_spaced_out_at_least_100_mills = timestamps
                .iter()
                .zip(timestamps.iter().skip(1))
                .all(|(current_timestamp, the_next_timestamp)| {
                    *the_next_timestamp - *current_timestamp >= Duration::from_millis(100)
                });

            assert!(timestamps_spaced_out_at_least_100_mills, "It seems that retry didn't allow for the allotted time to pass between two attempts");

            Ok(())
        }
    }
}
