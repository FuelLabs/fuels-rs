# Logs

Whenever you log a value within a contract method, the resulting log entry is added to the log receipt and the variable type is recorded in the contract's ABI. The SDK lets you parse those values into Rust types.

Consider the following contract method:

```rust,ignore
{{#include ../../../e2e/sway/logs/contract_logs/src/main.sw:produce_logs}}
```

You can access the logged values in Rust by calling `decode_logs_with_type::<T>` from a `FuelCallResponse`, where `T` is the type of the logged variables you want to retrieve. The result will be a `Vec<T>`:

```rust,ignore
{{#include ../../../e2e/tests/logs.rs:produce_logs}}
```

You can use the `decode_logs()` function to retrieve a `LogResult` struct containing a `results` field that is a vector of `Result<String>` values representing the success or failure of decoding each log.

```rust, ignore
{{#include ../../../e2e/tests/logs.rs:decode_logs}}
```

Due to possible performance hits, it is not recommended to use `decode_logs()` outside of a debugging scenario.

> **Note:** String slices cannot be logged directly. Use the `__to_str_array()` function to convert it to a `str[N]` first.
