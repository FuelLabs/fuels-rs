# Calling contracts

Once you've deployed your contract, as seen in the previous sections, you'll likely want to:

1. Call contract methods;
2. Configure call and transaction parameters such as gas price, byte price, and gas limit;
3. Forward coins and gas in your contract calls;
4. Read and interpret returned values and logs.

Here's an example. Suppose your Sway contract has two ABI methods called `initialize_counter(u64)` and `increment_counter(u64)`. Once you've deployed it the contract, you can call these methods like this:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:use_deployed_contract}}
```

The example above uses all the default configurations and performs a simple contract call.

Next, we'll see how we can further configure the many different parameters in a contract call
