use std::future::Future;
use std::time::Duration;

pub async fn retry_until<Fut>(
    predicate: impl Fn() -> Fut,
    max_attempts: usize,
    interval: Duration,
) -> anyhow::Result<bool>
where
    Fut: Future<Output = anyhow::Result<bool>>,
{
    for _ in 0..max_attempts {
        if predicate().await? {
            return Ok(true);
        }
        tokio::time::sleep(interval).await;
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use crate::utils::retry_until;
    use anyhow::bail;
    use std::time::{Duration, Instant};
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn retry_until_will_try_the_requested_amount_of_times() -> anyhow::Result<()> {
        let counter = Mutex::new(0);

        let will_always_fail = || async {
            *counter.lock().await += 1;

            Ok(false)
        };

        retry_until(will_always_fail, 5, Duration::from_secs(0)).await?;

        assert_eq!(*counter.lock().await, 5);

        Ok(())
    }

    #[tokio::test]
    async fn retry_until_will_return_false_if_attempts_exhausted() -> anyhow::Result<()> {
        let will_always_fail = || async { Ok(false) };

        let response = retry_until(will_always_fail, 2, Duration::from_secs(0)).await?;

        assert!(!response);

        Ok(())
    }

    #[tokio::test]
    async fn retry_until_will_respect_delay_between_attempts() -> anyhow::Result<()> {
        let timestamps_predicate_was_called_at: Mutex<Vec<Instant>> = Mutex::new(vec![]);

        let will_fail = || async {
            timestamps_predicate_was_called_at
                .lock()
                .await
                .push(Instant::now());
            Ok(false)
        };

        retry_until(will_fail, 2, Duration::from_millis(250)).await?;

        let timestamps = timestamps_predicate_was_called_at.lock().await.clone();

        let timestamps_spaced_out_at_least_250_mills = timestamps
            .iter()
            .zip(timestamps.iter().skip(1))
            .all(|(current_timestamp, the_next_timestamp)| {
                *the_next_timestamp - *current_timestamp >= Duration::from_millis(250)
            });

        assert!(timestamps_spaced_out_at_least_250_mills, "It seems that retry_until didn't allow for the allotted time to pass between two attempts");

        Ok(())
    }

    #[tokio::test]
    async fn retry_until_will_fail_fast_in_case_of_error_in_predicate() -> anyhow::Result<()> {
        let times_called = Mutex::new(0);
        let will_fail_w_error = || async {
            *times_called.lock().await += 1;
            bail!("Some error")
        };

        let response = retry_until(will_fail_w_error, 5, Duration::from_secs(0)).await;

        let err = response.expect_err("Should have propagated the failure of the predicate");

        assert_eq!(err.to_string(), "Some error");
        assert_eq!(*times_called.lock().await, 1);

        Ok(())
    }
}
