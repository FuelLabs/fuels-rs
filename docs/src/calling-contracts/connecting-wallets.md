# Connecting wallets

You can use the `_connect()` method on an existing contract instance as a shorthand for creating new instances connected to different wallets. This is especially useful when simulating calls to a contract initiated by multiple wallets.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:connect_wallet}}
```


**Note** `_connect()` returns a newly created contract instance which allows you to directly chain contract calls.