# Retry Mechanism

When we submit a transaction, it is dispatched to the txpool, where it awaits execution. If the submission is successful, we receive a transaction ID, which enables us to request transaction receipts or return values of submitted methods. However, it's crucial to acknowledge that during the transaction, `IO::Errors` may occur, potentially preventing the transaction from reaching the txpool.
In the event of a successful transaction, when attempting to retrieve its receipts or the return values of submitted methods, we may encounter `IO::Errors` or receive a `None` as the return value of these methods, indicating that the result is not yet available.

To address these scenarios, we can configure the number of retry attempts and the retry strategy for transaction submissions, as detailed further in the following section of this document.

## RetryConfig

The `RetryConfig` struct encapsulates the configuration parameters for controlling the retry behavior of asynchronous actions. It includes the maximum number of attempts and the interval strategy from the `Backoff` enum that determines how much time to wait between retry attempts.

```rust, ignore
{{#include ../../packages/fuels-programs/src/retry.rs:retry_config}}
```

## Backoff

The `Backoff` enum defines different strategies for managing intervals between retry attempts.
Each strategy allows you to customize the waiting time before a new attempt based on the number of attempts made.

### Variants

- `Linear(Duration)`: `Default` Increases the waiting time linearly with each attempt.
- `Exponential(Duration)`: Doubles the waiting time with each attempt.
- `Fixed(Duration)`: Uses a constant waiting time between attempts.

```rust, ignore
{{#include ../../packages/fuels-programs/src/retry.rs:backoff}}
```

## Transaction Workflow

### Submitting transaction

```rust, ignore
{{#include ../../examples/contracts/src/lib.rs:submit_retry}}
```

### Requesting values

In this step, we use the `response` obtained from the previous step to retrieve the desired values.

```rust, ignore
{{#include ../../examples/contracts/src/lib.rs:response_retry}}
```
