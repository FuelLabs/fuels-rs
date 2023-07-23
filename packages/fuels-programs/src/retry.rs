use std::future::Future;
use std::num::NonZeroUsize;
use std::time::Duration;

pub async fn retry<Fut, T, K>(
    action: impl Fn() -> Fut,
    interval: Duration,
    max_attempts: NonZeroUsize,
) -> Result<T, K>
where
    Fut: Future<Output = Result<T, K>>,
{
    let mut last_err = None;

    for _ in 0..max_attempts.get() {
        match action().await {
            Ok(value) => return Ok(value),
            Err(error) => last_err = Some(error),
        }
        tokio::time::sleep(interval).await;
    }

    Err(last_err.expect("Must have failed"))
}

#[cfg(test)]
mod tests {
    mod retry_until {
        use crate::retry::retry;
        use anyhow::anyhow;
        use std::time::{Duration, Instant};
        use tokio::sync::Mutex;

        #[tokio::test]
        async fn gives_up_after_max_attempts() -> anyhow::Result<()> {
            let number_of_attempts = Mutex::new(0usize);

            let will_always_fail = || async {
                *number_of_attempts.lock().await += 1;

                Result::<(), _>::Err(anyhow!("Error"))
            };

            let _ = retry(
                will_always_fail,
                Duration::from_millis(10),
                3.try_into().unwrap(),
            )
            .await;

            assert_eq!(*number_of_attempts.lock().await, 3);

            Ok(())
        }

        #[tokio::test]
        async fn returns_last_error() -> anyhow::Result<()> {
            let err_msgs = ["Err1", "Err2", "Err3"];
            let number_of_attempts = Mutex::new(0usize);

            let will_always_fail = || async {
                let msg = err_msgs[*number_of_attempts.lock().await];
                *number_of_attempts.lock().await += 1;

                Result::<(), _>::Err(anyhow!(msg))
            };

            let err = retry(
                will_always_fail,
                Duration::from_millis(10),
                3.try_into().unwrap(),
            )
            .await
            .expect_err("Should have failed");

            assert_eq!(err.to_string(), err_msgs[2]);

            Ok(())
        }

        #[tokio::test]
        async fn returns_value_on_success() -> anyhow::Result<()> {
            let values = Mutex::new(vec![
                Ok(String::from("Success")),
                Err(anyhow!(String::from("Err1"))),
                Err(anyhow!(String::from("Err2"))),
            ]);

            let will_always_fail = || async { values.lock().await.pop().unwrap() };

            let ok = retry(
                will_always_fail,
                Duration::from_millis(10),
                3.try_into().unwrap(),
            )
            .await?;

            assert_eq!(ok, "Success");

            Ok(())
        }

        #[tokio::test]
        async fn retry_respects_delay_between_attempts() -> anyhow::Result<()> {
            let timestamps: Mutex<Vec<Instant>> = Mutex::new(vec![]);

            let will_fail_and_record_timestamp = || async {
                timestamps.lock().await.push(Instant::now());
                Result::<(), _>::Err(anyhow!("Error"))
            };

            let _ = retry(
                will_fail_and_record_timestamp,
                Duration::from_millis(100),
                3.try_into().unwrap(),
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
