# Predicate data

Let's consider the following predicate example:

```rust,ignore
{{#include ../../../packages/fuels/tests/predicates/predicate_data_example/src/main.sw}}
```

With the Fuel Rust SDK, You can encode and send the predicate data through `Predicate`'s `encode_data()`:

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:encode_predicate_data}}
```

Keep on reading for the full example.

Notice how this predicate uses `input_predicate_data()`, a way for the predicate code to read the data the caller passed to it.

Like everything else in the FuelVM, this data follows the ABI encoding/decoding specification. When using the Fuel Rust SDK to pass data to this predicate, you must encode it properly.

Here's how you can do it. First, we set up the wallets, node, and predicate code:

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_data_setup}}
```

Next, we lock some assets in this predicate using the first wallet:

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_data_lock_amount}}
```

Then, we try to unlock the amount and spend it using the second wallet, effectively sending the previously locked value to itself.

The predicate expects the data sent to it to be a `u64` type with the value `42`.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_data_unlock}}
```

> **Note:** if the data you're encoding is already a `Vec<u8>`, e.g., in the [send and spend examples](./send-spend-predicate.md), then you don't need to call `encode_predicate_data()`, passing it as-is works.
