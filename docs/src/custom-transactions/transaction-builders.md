# Transaction Builders

The Rust SDK simplifies the creation of **Create** and **Script** transactions through two handy builder structs `CreateTransactionBuilder`, `ScriptTransactionBuilder`, and the `TransactionBuilder` trait.

Calling `build()` on a builder will result in the corresponding `CreateTransaction` or `ScriptTransaction` that can be submitted to the network.

## Role of the transaction builders

> **Note** This section contains additional information about the inner workings of the builders. If you are just interested in how to use them, you can skip to the next section.

The builders take on the heavy lifting behind the scenes, offering two standout advantages: handling predicate data offsets and managing witness indexing.

When your transaction involves predicates with dynamic data as inputs, like vectors, the dynamic data contains a pointer pointing to the beginning of the raw data. This pointer's validity hinges on the order of transaction inputs, and any shifting could render it invalid. However, the transaction builders conveniently postpone the resolution of these pointers until you finalize the build process.

Similarly, adding signatures for signed coins requires the signed coin input to hold an index corresponding to the signature in the witnesses array. These indexes can also become invalid if the witness order changes. The Rust SDK again defers the resolution of these indexes until the transaction is finalized. It handles the assignment of correct index witnesses behind the scenes, sparing you the hassle of dealing with indexing intricacies during input definition.

Another added benefit of the builder pattern is that it guards against changes once the transaction is finalized. The transactions resulting from a builder don't permit any changes to the struct that could cause the transaction ID to be modified. This eliminates the headache of calculating and storing a transaction ID for future use, only to accidentally modify the transaction later, resulting in a different transaction ID.

## Creating a custom transaction

Here is an example outlining some of the features of the transaction builders.

In this scenario, we have a predicate that holds some bridged asset with ID **bridged_asset_id**. It releases it's locked assets if the transaction sends **ask_amount** of the base asset to the **receiver** address:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_tx_receiver}}
```

Our goal is to create a transaction that will use our hot wallet to transfer the **ask_amount** to the **receiver** and then send the unlocked predicate assets to a second wallet that acts as our cold storage.

Let's start by instantiating a builder. Since we don't plan to deploy a contract, the `ScriptTransactionBuilder` is the appropriate choice:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_tx}}
```

Next, we need to define transaction inputs of the base asset that sum up to **ask_amount**. We also need transaction outputs that will assign those assets to the predicate address and thereby unlock it. The methods `get_asset_inputs_for_amount` and `get_asset_outputs_for_amount` can help with that. We need to specify the asset ID, the target amount, and the target address:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_tx_io_base}}
```

Let's repeat the same process but this time for transferring the assets held by the predicate to our cold storage:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_tx_io_other}}
```

We combine all of the inputs and outputs and set them on the builder:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_tx_io}}
```

As we have used coins that require a signature, we sign the transaction builder with:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_tx_sign}}
```

> **Note** The signature is not created until the transaction is finalized with `build(&provider)`

We need to do one more thing before we stop thinking about transaction inputs. Executing the transaction also incurs a fee that is paid with the base asset. Our base asset inputs need to be large enough so that the total amount covers the transaction fee and any other operations we are doing. The Account trait lets us use `adjust_for_fee()` for adjusting the transaction inputs if needed to cover the fee. The second argument to `adjust_for_fee()` is the total amount of the base asset that we expect our transaction to spend regardless of fees. In our case, this is the **ask_amount** we are transferring to the predicate.

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_tx_adjust}}
```

We can also define transaction policies. For example, we can limit the gas price by doing the following:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_tx_policies}}
```

Our builder needs a signature from the hot wallet to unlock its coins before we call `build()` and submit the resulting transaction through the provider:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_tx_build}}
```

Finally, we verify the transaction succeeded and that the cold storage indeed holds the bridged asset now:

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_tx_verify}}
```
