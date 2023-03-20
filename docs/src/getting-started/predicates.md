# Predicates

Predicates, in Sway, are programs that return a Boolean value, and they do not have any side effects (they are pure).

## Instantiating predicates

Once you've written a predicate in Sway and compiled it with `forc build`, you can use the `abigen!` to generate all the types specified in the predicate. Additionally, you will get a `PredicateEncoder` instance with an `encode_data()` method for encoding the predicate data. You can then load the corresponding `Predicate` from its binary, set its data, and use it to receive and spend assets via the [Account](../getting-started/account.md) trait. The code snippet below shows how to use the abigen and generate a predicate instance.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_load}}
```
The predicate address is generated from the compiled byte code and is the same as the `P2SH` address used in Bitcoin. Users can seamlessly send assets to the predicate address as they do for any other address on the chain. To spend the predicate funds, the user has to provide the original `byte code` of the predicate together with the `predicate data`. The `predicate data` will be used when executing the `byte code`, and if the predicate is validated successfully, the funds can be transferred.

In the next section, we show how to interact with a predicate and explore an example where specific signatures are needed to spend the predicate funds.
