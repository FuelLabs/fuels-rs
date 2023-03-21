# Predicates

Predicates, in Sway, are programs that return a Boolean value, and they do not have any side effects (they are pure).

## Instantiating predicates

Once you've written a predicate in Sway and compiled it with `forc build`, you can load the predicate using `Predicate::load_from`. 

If you need to encode predicate data you will need to run the `abigen!` macro.

This will generate all the types specified in the predicate plus a custom encoder with an `encode_data` function.
 
The resulting data can then be set on the loaded predicate.

Predicates can be used to receive and spend assets via the [Account](../getting-started/account.md) trait. The code snippet below shows how to load a predicate, encode its data and use it as an `Account`.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_load}}
```

The predicate address is generated from the compiled byte code and is the same as the `P2SH` address used in Bitcoin. Users can seamlessly send assets to the predicate address as they do for any other address on the chain. To spend the predicate funds, the user has to provide the original `byte code` of the predicate together with the `predicate data`. The `predicate data` will be used when executing the `byte code`, and if the predicate is validated successfully, the funds can be transferred.

In the next section, we show how to interact with a predicate and explore an example where specific signatures are needed to spend the predicate funds.
