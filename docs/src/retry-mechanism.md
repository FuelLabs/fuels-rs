# Retry Mechanism

When we submit a transaction, it is dispatched to the txpool, where it awaits execution. If the submission is successful, we receive a transaction ID, which enables us to request transaction receipts or its value. However,
it's crucial to acknowledge that during the transaction, `IO::Errors` may occur, potentially preventing the transaction from reaching the txpool.

In the event of a successful transaction, when attempting to retrieve its receipts or values, we may encounter `IO::Errors` or receive a `None` value, indicating that the result is not yet available.

To address these scenarios, we can configure the number of retry attempts and the retry strategy for transaction submissions, as detailed further in the following section of this document.

## RetryConfig

The `RetryConfig` struct encapsulates the configuration parameters for controlling the retry behavior
of asynchronous actions. It includes the maximum number of attempts and the interval strategy from
the `Backoff` enum that determines how much time to wait between retry attempts.

```rust, ignore
#[derive(Clone, Debug, Default)]
pub struct RetryConfig {
    pub max_attempts: usize,
    pub interval: Backoff,
}
```

## Backoff

The `Backoff` enum defines different strategies for managing intervals between retry attempts.
Each strategy allows you to customize the waiting time before a new attempt based on the
number of attempts made.

### Variants

- `Linear(Duration)`: `Default` Increases the waiting time linearly with each attempt.
- `Exponential(Duration)`: Doubles the waiting time with each attempt.
- `Fixed(Duration)`: Uses a constant waiting time between attempts.

```rust, ignore
#[derive(Debug, Clone)]
pub enum Backoff {
    Linear(Duration),
    Exponential(Duration),
    Fixed(Duration),
}
```

## Transaction Workflow

### Submitting transaction

```rust, ignore
        let retry_config = RetryConfig::new(3, Backoff::default());
        let response = contract_instance
            .methods()
            .initialize_counter(42)
            .retry_config(retry_config)
            .submit()
            .await?;
```

### Requesting values

In this step, we use the `response` obtained from the previous step to retrieve the desired values.

```rust, ignore
        let retry_config = RetryConfig::new(5, Backoff::default());
        let value = response.retry_config(retry_config).value().await?;
```
