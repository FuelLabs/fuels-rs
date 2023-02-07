# Call parameters

Call parameters are:

1. Amount;
2. Asset ID;
3. Gas forwarded.

You can use these to forward coins to a contract. You can configure these parameters by creating an instance of [`CallParameters`](https://docs.rs/fuels/latest/fuels/core/parameters/struct.CallParameters.html#) and passing it to a chain method called `call_params`.

For instance, suppose the following contract that uses Sway's `msg_amount()` to return the amount sent in that transaction.

```rust,ignore
{{#include ../../../packages/fuels/tests/contracts/contract_test/src/main.sw:msg_amount}}
```

Then, in Rust, after setting up and deploying the above contract, you can configure the amount being sent in the transaction like this:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:call_parameters}}
```

`call_params` returns a result to ensure you don't forward assets to a contract method that isn't payable. In the following example, we try to forward an amount of 100 of the base asset to `non_payable`. As its name suggests, `non_payable` isn't annotated with `#[payable]` in the contract code. Passing `CallParameters` with an amount other than 0 leads to an `InvalidCallParameters` error:

```rust,ignore
{{#include ../../../packages/fuels/tests/contracts.rs:non_payable_params}}
```

> **Note:** forwarding gas to a contract call is always possible, regardless of the contract method being non-payable.

You can also use `CallParameters::default()` to use the default values:

```rust,ignore
{{#include ../../../packages/fuels-core/src/constants.rs:default_call_parameters}}
```

This way:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:call_parameters_default}}
```

The `gas_forwarded` parameter defines the limit for the actual contract call as opposed to the gas limit for the whole transaction. This means that it is constrained by the transaction limit. If it is set to an amount greater than the available gas, all available gas will be forwarded.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:call_params_gas}}
```

If you don't set the call parameters or use `CallParameters::default()`, the transaction gas limit will be forwarded instead.
