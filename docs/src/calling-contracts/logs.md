# Logs

Whenever you log a value within a contract method, the resulting log entry is added to the log receipt and the variable type is recorded in the contract's ABI. The SDK lets you parse those values into Rust types.

Consider the following contract method:

```rust,ignore
{{#include ../../../packages/fuels/tests/logs/logged_types/src/main.sw:produce_logs}}
```

You can access the logged values in Rust by calling `logs_with_type::<T>` from a `CallResponse`, where `T` is the type of the logged variables you want to retrieve. The result will be a `Vec<T>`:

```rust,ignore
{{#include ../../../packages/fuels/tests/logs.rs:produce_logs}}
```

You can also get a vector of all the logged values as strings using `get_logs()`:

```rust, ignore
{{#include ../../../packages/fuels/tests/logs.rs:get_logs}}
```

Due to possible performance hits, it is not recommended to use `get_logs()` outside of a debugging scenario.
