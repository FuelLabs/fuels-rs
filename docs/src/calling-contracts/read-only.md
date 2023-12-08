# Read-only calls

<!-- This section should explain read-only calls  -->
<!-- read_only:example:start -->
Sometimes you want to call a contract method that doesn't change the state of the blockchain. For instance, a method that only reads a value from storage and returns it.

In this case, there's no need to generate an actual blockchain transaction; you only want to read a value quickly.

You can do this with the SDK. Instead of calling the method with `.call()`, use `.simulate()`:
<!-- read_only:example:end -->

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:simulate}}
```

<!-- This section should explain what happens if you try a read-only call on a method that changes state  -->
<!-- simulate:example:start -->
Note that if you use `.simulate()` on a method that _does_ change the state of the blockchain, it won't work properly; it will just `dry-run` it.

At the moment, it's up to you to know whether a contract method changes state or not, and use `.call()` or `.simulate()` accordingly.
<!-- simulate:example:end -->
