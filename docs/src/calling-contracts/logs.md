# Logs

Whenever you log a value within a contract call, the resulting log entry is added to the receipt and the variable type is recorded in the contracts ABI. The SDK lets you parse those values into Rust types.

Consider the following contract method:

```rust,ignore
{{#include ../../../packages/fuels/tests/test_projects/logged_types/src/main.sw:produce_logs}}
```

You can access the logged values in Rust by calling `_logs_with_type::<T>` from a contract instance, where `T` is the type of the logged variables you want to retrieve. The result will be a `Vec<T>`:

```rust,ignore
{{#include ../../../packages/fuels/tests/harness.rs:produce_logs}}
```

> **Note:** to be able to bind logged values in the SDK, you need to build your contract by supplying a feature flag: `forc build --generate-logged-types`.
