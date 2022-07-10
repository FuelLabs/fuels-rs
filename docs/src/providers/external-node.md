# Connecting to an external Fuel node

If you want your SDK code to connect to an already running Fuel node, which could be devnet, testnet, mainnet, or a local instance through `fuel-core`, this is how you do it:

```rust,ignore
{{#include ../../../examples/providers/src/lib.rs:connect_to_node}}
```
