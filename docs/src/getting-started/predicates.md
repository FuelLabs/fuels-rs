# Predicates

Predicates, in Sway, are programs that return a Boolean value and do not have any side effects (they are pure). A predicate address can own assets. The predicate address is generated from the compiled byte code and is the same as the `P2SH` address used in Bitcoin. Users can seamlessly send assets to the predicate address as they do for any other address. To spend the predicate funds, the user has to provide the original `byte code` of the predicate together with the `predicate data`. The `predicate data` will be used when executing the `byte code`, and the funds can be transferred if the predicate is validated successfully.

## Instantiating predicates

Let's consider the following predicate example:

```rust,ignore
{{#include ../../../packages/fuels/tests/predicates/basic_predicate/src/main.sw}}
```

We will look at a complete example of using the SDK to send and receive funds from a predicate.

First, we set up the wallets, node, and a predicate encoder instance. The call to the `abigen!` macro will generate all the types specified in the predicate plus a custom encoder with an `encode_data` function that will conveniently encode all the arguments of the main function for us.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_data_setup}}

Once we've compiled our predicate with `forc build`, we can create a `Predicate` instance via `Predicate::load_from`. The resulting data from `encode_data` can then be set on the loaded predicate.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:with_predicate_data}}
```

Next, we lock some assets in this predicate using the first wallet:

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_data_lock_amount}}
```

Then we can transfer assets owned by the predicate via the [Account](../getting-started/account.md) trait:

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_data_unlock}}
```
