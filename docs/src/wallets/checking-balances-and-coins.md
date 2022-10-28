# Checking balances and coins

First, one should remember that, with UTXOs, each _coin_ is unique. Each UTXO corresponds to a unique _coin_, and said _coin_ has a corresponding _amount_ (the same way a dollar bill has either 10$ or 5$ face value). So, when you want to query the balance for a given asset ID, you want to query the sum of the amount in each unspent coin. This querying is done very easily with a wallet:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:get_asset_balance}}
```
If you want to get all spendable coins, you can use:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:get_spendable_coins}}
```

If you want to query all the balances (i.e., get the balance for each asset ID in that wallet), you can use:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:get_balances}}
```

`get_balances(num_results: u64)` is a paginated request and the input is the number of results that you want to have per page. The `Pager` struct can be used to set the `cursor` and the pagination direction. To get the actual results, you have to use the `call()` method. The return type of `call()` is a `Page` struct. The `Page` struct includes the current `cursor`, `results` and information on whether we have the next and previous page (`has_next_page`, `has_previous_page`).

In this case, the `results` variable is a `HashMap`, where the key is an `AssetId`, and the value is the corresponding balance. For example, we can get the base asset balance with:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:get_balance_hashmap}}
```

> **Note:** That `get_coins`, `get_messages` and `get_transactions` are also paginated requests and are used in the same way as `get_balances`.
