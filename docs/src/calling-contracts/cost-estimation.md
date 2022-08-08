# Estimating contract call cost

With with the function `get_transaction_cost(tolerance: Option<f64>)` provided by `ContractCallHandler` and `ContractMultiCallHandler`, you can get a cost estimation for the specific call. The return type, `TransactionCost`, is a struct that contains relevant information for the call cost:


```rust,ignore
TransactionCost {
    min_gas_price: u64,
    min_byte_price: u64,
    byte_price: u64,
    gas_price: u64,
    gas_used: u64,
    byte_size: u64,
    fee: u64, // where fee is the sum of the gas and byte fees
}
```

Below are examples how to get the estimate transaction cost from single and multi call transactions.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:contract_call_cost_estimation}}
```

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_call_cost_estimation}}
```

The transaction cost estimation can be used to set the gas limit for the actual call, or to show the user the estimated cost.

