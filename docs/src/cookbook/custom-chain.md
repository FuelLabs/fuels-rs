# Custom chain

This example demonstrates how to start a short-lived Fuel node with custom consensus parameters for the underlying chain.

First, we have to import `ConsensusParameters` from the fuels-tx crate:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_chain_import}}
```

Next, we can define some values for the consensus parameters:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_chain_consensus}}
```

Before we can start a node, we probably also want to define some genesis coins and assign them to an address:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_chain_coins}}
```

Finally, we call `setup_test_client()`, which starts a node with the given configs and returns a client:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_chain_client}}
```
