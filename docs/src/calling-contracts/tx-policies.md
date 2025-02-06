# Transaction policies

<!-- This section should explain what tx policies are and how to configure them -->
<!-- tx_policies:example:start -->
Transaction policies are defined as follows:

```rust,ignore
{{#include ../../../packages/fuels-core/src/types/wrappers/transaction.rs:tx_policies_struct}}
```

Where:

1. **Tip** - amount to pay the block producer to prioritize the transaction.
2. **Witness Limit** - The maximum amount of witness data allowed for the transaction.
3. **Maturity** - Block until which the transaction cannot be included.
4. **Expiration** - Block after which the transaction cannot be included.
5. **Max Fee** - The maximum fee payable by this transaction.
6. **Script Gas Limit** - The maximum amount of gas the transaction may consume for executing its script code.

When the **Script Gas Limit** is not set, the Rust SDK will estimate the consumed gas in the background and set it as the limit.

If the **Witness Limit** is not set, the SDK will set it to the size of all witnesses and signatures defined in the transaction builder.

You can configure these parameters by creating an instance of `TxPolicies` and passing it to a chain method called `with_tx_policies`:
<!-- tx_policies:example:end-->

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:tx_policies}}
```

<!-- This section should explain how to use the default tx policy -->
<!-- tx_policies_default:example:start -->
You can also use `TxPolicies::default()` to use the default values.
<!-- tx_policies_default:example:end -->

This way:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:tx_policies_default}}
```

As you might have noticed, `TxPolicies` can also be specified when deploying contracts or transferring assets by passing it to the respective methods.
