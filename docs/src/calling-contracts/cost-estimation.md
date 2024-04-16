# Estimating contract call cost

With the function `estimate_transaction_cost(tolerance: Option<f64>, block_horizon: Option<u32>)` provided by `ContractCallHandler` and `MultiContractCallHandler`, you can get a cost estimation for a specific call. The return type, `TransactionCost`, is a struct that contains relevant information for the estimation:

```rust,ignore
{{#include ../../../packages/fuels-accounts/src/provider.rs:transaction_cost}}
```

Below are examples that show how to get the estimated transaction cost from single and multi call transactions.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:contract_call_cost_estimation}}
```

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_call_cost_estimation}}
```

The transaction cost estimation can be used to set the gas limit for an actual call, or to show the user the estimated cost.

> **Note** The same estimation interface is available for scripts.
