# Signatures in predicates example

This is a more involved example where the predicate accepts three signatures and matches them to three predefined public keys. The `ec_recover_address` function is used to recover the public key from the signatures. If two of the three extracted public keys match the predefined public keys, the funds can be spent. Note that the signature order has to match the order of the predefined public keys.

```rust,ignore
{{#include ../../../packages/fuels/tests/predicates/predicate_signatures/src/main.sw}}
```

Let's use the SDK to interact with the predicate. First, let's create three wallets with specific keys. Their hashed public keys are already hard-coded in the predicate. Then we create the receiver wallet, which we will use to spend the predicate funds.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_wallets}}
```

Next, let's add some coins, start a provider and connect it with the wallets.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_coins}}
```

Now we can use the predicate abigen to create a predicate encoder instance for us. To spend the funds now locked in the predicate, we must provide two out of three signatures whose public keys match the ones we defined in the predicate. In this example, the signatures are generated from an array of zeros.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_load}}
```

Next, we transfer some assets from a wallet to the created predicate. We also confirm that the funds are indeed transferred.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_receive}}
```

We can use the `transfer` method from the [Account](../getting-started/account.md) trait to transfer the assets. If the predicate data is correct, the `receiver` wallet will get the funds, and we will verify that the amount is correct.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_spend}}
```