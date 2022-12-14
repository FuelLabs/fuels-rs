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

Now we can use the predicate abigen, which will create a predicate instance for us.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_load}}
```

After the predicate instance is generated we can use the `receive` function to transfer funds to the predicate. We also make sure that the funds are indeed transferred.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_receive}}
```

To spend the funds that are now locked in the predicate, we have to provide two out of three signatures whose public keys match the ones we defined in the predicate. In this example, the signatures are generated from an array of zeros.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_signatures}}
```

After generating the signatures, we can use the predicate's `encode_data` and `spend` functions to spend the funds. If the provided data is correct the `receiver` wallet will get the funds, and we will verify that the amount is indeed correct.

```rust,ignore
{{#include ../../../examples/predicates/src/lib.rs:predicate_spend}}
```
