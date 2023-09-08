# Retry Mechanism

When we submit a transaction, it is dispatched to the txpool, where it awaits execution. If the submission is successful, we receive a transaction ID, which enables us to request transaction receipts (and parse them) once the transaction is committed. However, it's crucial to acknowledge `IO::Errors` may occur, potentially preventing the transaction from reaching the txpool.
In the event of a successful transaction, when attempting to retrieve its receipts, we may encounter `IO::Errors` or network request might pass but the receipts haven't propagated to the node yet.

To address these scenarios, we can configure the number of retry attempts and the retry strategy for transaction submissions/responses, as detailed below.

## RetryConfig

The retry behavior can be altered by giving a custom `RetryConfig`. It allows for configuring the maximum number of attempts and the interval strategy used.

```rust, ignore
{{#include ../../packages/fuels-programs/src/retry.rs:retry_config}}
```

## Interval strategy - Backoff

`Backoff` defines different strategies for managing intervals between retry attempts.
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
