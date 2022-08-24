# Interacting with contracts

If you already have a deployed contract and want to call its methods using the SDK,  but without deploying it again, all you need is the contract ID of your deployed contract. You can skip the whole deployment setup and call `::new(contract_id, wallet)` directly. For example:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deployed_contracts}}
