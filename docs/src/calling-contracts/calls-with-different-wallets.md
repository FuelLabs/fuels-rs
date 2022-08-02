# Calls with different wallets

You can use the `_with_wallet()` method on an existing contract instance as a shorthand for creating a new instance connected to the provided wallet. This lets you make contracts calls with different wallets in a chain like fashion.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:connect_wallet}}
```
