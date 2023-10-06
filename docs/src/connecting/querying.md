# Querying the blockchain

Once you set up a provider, you can interact with the Fuel blockchain. Here are a few examples of what you can do with a provider; for a more in-depth overview of the API, check the [official provider API documentation](https://docs.rs/fuels/{{versions.fuels}}/fuels/accounts/provider/struct.Provider.html).

- [Set up](#set-up)
- [Get all coins from an address](#get-all-coins-from-an-address)
- [Get spendable resources owned by an address](#get-spendable-resources-owned-by-an-address)
- [Get balances from an address](#get-balances-from-an-address)

## Set up

You might need to set up a test blockchain first. You can skip this step if you're connecting to an external blockchain.

```rust,ignore
{{#include ../../../examples/providers/src/lib.rs:setup_test_blockchain}}
```

## Get all coins from an address

This method returns all unspent coins (of a given asset ID) from a wallet.

```rust,ignore
{{#include ../../../examples/providers/src/lib.rs:get_coins}}
```

## Get spendable resources owned by an address

The following example shows how to fetch resources owned by an address. First, you create a  `ResourceFilter` which specifies the target address, asset id, and amount. You can also define utxo ids and message ids that should be excluded when retrieving the resources:

```rust,ignore
{{#include ../../../packages/fuels-accounts/src/provider.rs:resource_filter}}
```

The example uses default values for the asset id and the exclusion lists. This resolves to the base asset id and empty vectors for the id lists respectively:

```rust,ignore
{{#include ../../../examples/providers/src/lib.rs:get_spendable_resources}}
```

## Get balances from an address

Get all the spendable balances of all assets for an address. This is different from getting the coins because we only return the numbers (the sum of UTXOs coins amount for each asset id) and not the UTXOs coins themselves.

```rust,ignore
{{#include ../../../examples/providers/src/lib.rs:get_balances}}
```
