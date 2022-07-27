# Setting up test wallets

You'll often want to create one or more test wallets when testing your contracts. Here's how to do it.

## Setting up multiple test wallets

If you need multiple test wallets, they can be set up as follows:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:multiple_wallets_helper}}
```

You can customize your test wallets via `WalletsConfig`.

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:setup_5_wallets}}
```

## Setting up a test wallet with multiple random assets

You can create a test wallet containing multiple assets (including the base asset to pay for gas).

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:multiple_assets_wallet}}
```

- coins: `Vec<(UtxoId, Coin)>` has num_assets * coins_per_assets coins (UTXOs)
- asset_ids: `Vec<AssetId>` contains the num_assets randomly generated `AssetId`s (always includes the base asset)

## Setting up a test wallet with multiple custom assets

You can also create assets with specific `AssetId`s, coin amounts, and number of coins.

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:custom_assets_wallet}}
```

>**Note** In this case, you need to specify also the base asset and the corresponding number of
>coins and coin amount

## Setting up assets

The Fuel blockchain holds many different assets; you can create your asset with its unique `AssetId` or create random assets for testing purposes.

You can use only one asset to pay for transaction fees and gas: the base asset, whose AssetId is `0x000...0`, a 32-byte zeroed value.

For testing purposes, you can configure coins and amounts for assets. You can use `setup_multiple_assets_coins`:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:multiple_assets_coins}}
```

>**Note** If setting up multiple assets, one of these assets will always be the base asset.

If you want to create coins only with the base asset, then you can use:

```rust,ignore
{{#include ../../../examples/providers/src/lib.rs:setup_single_asset}}
```
