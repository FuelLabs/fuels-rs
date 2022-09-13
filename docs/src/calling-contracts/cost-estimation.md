# Estimating contract call cost

With with the function `estimate_transaction_cost(tolerance: Option<f64>)` provided by `ContractCallHandler` and `ContractMultiCallHandler`, you can get a cost estimation for a specific call. The return type, `TransactionCost`, is a struct that contains relevant information for the estimation:


```rust,ignore
TransactionCost {
    min_gas_price: u64,
    min_byte_price: u64,
    gas_price: u64,
    gas_used: u64,
    metered_bytes_size: u64,
    total_fee: f64, // where total_fee is the sum of the gas and byte fees
}
```

Below are examples that show how to get the estimated transaction cost from single and multi call transactions.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:contract_call_cost_estimation}}
```

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_call_cost_estimation}}
```

The transaction cost estimation can be used to set the gas limit for an actual call, or to show the user the estimated cost.
