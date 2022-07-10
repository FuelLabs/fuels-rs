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

## Setting up a test wallet with multiple assets

You can create a test wallet containing multiple assets (including the base asset to pay for gas).

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:multiple_assets_wallet}}
```

- coins: `Vec<(UtxoId, Coin)>` has num_assets * coins_per_assets coins (UTXOs)
- asset_ids: `Vec<AssetId>` contains the num_assets randomly generated `AssetId`s (always includes the base asset)
