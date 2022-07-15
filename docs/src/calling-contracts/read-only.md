# Read-only calls

Sometimes you want to call a contract method that doesn't change the state of the blockchain. For instance, a method that only reads a value from storage and returns it.

In this case, there's no need to generate an actual blockchain transaction; you only want to read a value quickly.

You can do this with the SDK. Instead of `.call()`ing the method, use `.simulate()`:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:simulate}}
```

Note that if you use `.simulate()` on a method that _does_ change the state of the blockchain, it won't work properly; it will just `dry-run` it.

At the moment, it's up to you to know whether a contract method changes state or not, and use `.call()` or `.simulate()` accordingly.
