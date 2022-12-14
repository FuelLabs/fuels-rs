# Predicate data

Let's consider the following predicate example:

```rust,ignore
{{#include ../../../packages/fuels/tests/predicates/predicate_basic/src/main.sw}}
```

Similarly to contracts and scripts, the `predicate_abigen!` generates a function that will conveniently encode all the arguments of the main function for us. This function is called `encode_data`, and it is accessed through the predicate instance as shown below:

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:encode_predicate_data}}
```

Next, we will look at a complete example of using the SDK to send and receive funds from a predicate.


First, we set up the wallets, node, and a predicate instance:

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_data_setup}}
```

Next, we lock some assets in this predicate using the first wallet:

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_data_lock_amount}}
```

Then, we try to unlock the amount and spend it using the second wallet, effectively sending the previously locked value to itself.

The predicate expects the data sent to it to be two numbers (`u32` and `u64`) with matching values.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_data_unlock}}
```
