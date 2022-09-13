# Send and spend funds from predicates

Let's consider the following predicate example:

```rust,ignore
{{#include ../../../packages/fuels/tests/test_projects/predicate_signatures/src/main.sw}}
```

This predicate accepts three signatures and matches them to three predefined public keys. The `ec_recover_address` function is used to recover the public key from the signatures. If two of three extracted public keys match the predefined public keys, the funds can be spent. Note that the signature order has to match the order of the predefined public keys.

Let's use the SDK to interact with the predicate. First, let's create three wallets with specific keys. Their hashed public keys are already hard-coded in the predicate.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_wallets}}
```

Next, let's add some coins, start a provider and connect it with the wallets.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_coins}}
```

Now we can load the predicate binary, and prepare some transaction variables.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_load}}
```

After the predicate address is generated we can send funds to it. Note that we are using the same `transfer` function as we used when sending funds to other wallets. We also make sure that the funds are indeed transferred.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_send}}
```

To spend the funds that are now locked in the predicate, we have to provide two out of three signatures whose public keys match the ones we defined in the predicate. In this example, the signatures are generated from an array of zeros.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_signatures}}
```

After generating the signatures, we can send a transaction to spend the predicate funds. We use the `receiver` wallet as the recipient. We have to provide the predicate byte code and the required signatures. As we provide the correct data, we receive the funds and verify that the amount is correct.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_spend}}
```
