# Decoding script transactions

The SDK offers some tools that can help you make fuel script transactions more
human readable. You can determine whether the script transaction is:

* calling a contract method(s),
* is a loader script and you can see the blob id
* is neither of the above

In the case of contract call(s), if you have the ABI file, you can also decode
the arguments to the function by making use of the `AbiFormatter`:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:decoding_script_transactions}}
```

prints:

```text
The script called: initialize_counter(42)
```

The `AbiFormatter` can also decode configurables, refer to the rust docs for
more information.
