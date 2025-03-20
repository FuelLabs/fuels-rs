# Simulating calls

Sometimes you want to simulate a call to a contract without changing the state of the blockchain. This can be achieved by calling `.simulate` instead of `.call` and passing in the desired execution context:

* `.simulate(Execution::realistic())` simulates the transaction in a manner that closely resembles a real call. You need a wallet with base assets to cover the transaction cost, even though no funds will be consumed. This is useful for validating that a real call would succeed if made at that moment. It allows you to debug issues with your contract without spending gas.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:simulate}}
```

* `.simulate(Execution::state_read_only())` disables many validations, adds fake gas, extra variable outputs, blank witnesses, etc., enabling you to read state even with an account that has no funds.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:simulate_read_state}}
```

If the node supports historical execution (the node is using `rocksdb` and the `historical_execution` flag has been set), then both execution types can be chained with `at_height` to simulate the call at a specific block height.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:simulate_read_state_at_height}}
```
