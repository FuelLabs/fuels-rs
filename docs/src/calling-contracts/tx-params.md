# Transaction parameters

Transaction parameters are:

1. Gas price;
2. Gas limit;
3. Maturity.

You can configure these parameters by creating an instance of `TxParameters`:

```rust,ignore
{{#include ../../../packages/fuels-types/src/parameters.rs:tx_parameter}}
```

and passing it to a chain method called `tx_params`:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:tx_parameters}}
```

You can also use `TxParameters::default()` to use the default values:

```rust,ignore
{{#include ../../../packages/fuels-types/src/constants.rs:default_tx_parameters}}
```

This way:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:tx_parameters_default}}
```

As you might have noticed already, `TxParameters` can also be specified when deploying contracts or transfering assets by passing it to the respective methods.

> **Note:** whenever you perform an action that results in a transaction (contract deployment, contract call, asset transfer), the SDK will automatically estimate the fee based on the set gas limit and the transaction's byte size. This estimation is used when building the transaction. A side-effect of this is that your wallet must at least own a single coin of the base asset of any amount.
