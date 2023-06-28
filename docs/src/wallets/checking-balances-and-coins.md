# Checking balances and coins

<!-- This section should explain getting the balance of a wallet -->
<!-- balance:example:start -->
In the Fuel network, each UTXO corresponds to a unique _coin_, and said _coin_ has a corresponding _amount_ (the same way a dollar bill has either 10$ or 5$ face value). So, when you want to query the balance for a given asset ID, you want to query the sum of the amount in each unspent coin. This querying is done very easily with a wallet:
<!-- balance:example:end -->

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:get_asset_balance}}
```

<!-- This section should explain getting all of the balances of a wallet -->
<!-- balances:example:start -->
If you want to query all the balances (i.e., get the balance for each asset ID in that wallet), you can use the `get_balances` method:
<!-- balances:example:end -->

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:get_balances}}
```

<!-- This section should explain the return type for `get_balances` -->
<!-- balances_return:example:start -->
The return type is a `HashMap`, where the key is the _asset ID's_ hex string, and the value is the corresponding balance. For example, we can get the base asset balance with:
<!-- balances_return:example:end -->

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:get_balance_hashmap}}
```
