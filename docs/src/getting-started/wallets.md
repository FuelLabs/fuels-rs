# Managing Wallets

Wallets are used for many important things, for instance:

1. Checking your balance;
2. Transferring coins to a destination address;
3. Signing messages and transactions;
4. Paying for network fees when sending transactions or deploying smart contracts.

The SDK gives you many different ways to create wallets. Let's explore these different ways.

## Creating a wallet

A new wallet with a randomly generated private key can be created by supplying `Option<Provider>`.

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:create_random_wallet}}
```

Alternatively, you can create a wallet from a predefined `SecretKey`.

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:create_wallet_from_secret_key}}
```

> Note: if `None` is supplied instead of a provider, any transaction related to the wallet will result
> in an error until a provider is linked with `set_provider()`. The optional parameter
> enables defining owners (wallet addresses) of genesis coins before a provider is launched.

## Creating a wallet from a mnemonic phrase

A mnemonic phrase is a cryptographically-generated sequence of words that's used to derive a private key. For instance: `"oblige salon price punch saddle immune slogan rare snap desert retire surprise";` would generate the address `0xdf9d0e6c6c5f5da6e82e5e1a77974af6642bdb450a10c43f0c6910a212600185`.

In addition to that, we also support [Hierarchical Deterministic Wallets](https://www.ledger.com/academy/crypto/what-are-hierarchical-deterministic-hd-wallets) and [derivation paths](https://learnmeabitcoin.com/technical/derivation-paths). You may recognize the string `"m/44'/60'/0'/0/0"` from somewhere; that's a derivation path. In simple terms, it's a way to derive many wallets from a single root wallet.

The SDK gives you two wallet from mnemonic instantiation methods: one that takes a derivation path (`Wallet::new_from_mnemonic_phrase_with_path`) and one that uses the default derivation path, in case you don't want or don't need to configure that (`Wallet::new_from_mnemonic_phrase`).

Here's how you can create wallets with both mnemonic phrases and derivation paths:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:create_wallet_from_mnemonic}}
```

## Creating a wallet and storing an encrypted JSON wallet to disk

You can also manage a wallet using [JSON wallets](https://cryptobook.nakov.com/symmetric-key-ciphers/ethereum-wallet-encryption) -- wallets that are securely encrypted and stored on disk. This makes it easier to manage multiple wallets, especially for testing purposes.

You can create a random wallet and, at the same time, encrypt and store it. Then, later, you can recover the wallet if you know the master password:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:create_and_restore_json_wallet}}
```

## Encrypting and storing a wallet created from mnemonic or private key

If you had already created a wallet using a mnemonic phrase or a private key, you can also encrypt it and save it to disk:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:create_and_store_mnemonic_wallet}}
```

## Checking balances and coins

First, one should keep in mind that, with UTXOs, each _coin_ is unique. Each UTXO corresponds to a unique _coin_, and said _coin_ has a corresponding _amount_ (the same way a dollar bill has either 10$ or 5$ face value). So, when you want to query the balance for a given asset ID, you want to query the sum of the amount in each unspent coin. This is done very easily with a wallet:

```rust,ignore
let asset_id : AssetId = NATIVE_ASSET_ID
let balance : u64 = wallet.get_asset_balance(&asset_id).await;
```

If you want to query all the balances (i.e. get the balance for each asset IDs in that wallet), then it is as simple as:

```rust,ignore
let balances = wallet.get_balances().await.unwrap();
```

The return type is a `HashMap`, where the key is the _asset ID_ and the value is the corresponding balance.

## Security

Keep in mind that you should never share your private/secret key. And in the case of wallets that were derived from a mnemonic phrase, never share your mnemonic phrase.

If you're planning on storing the wallet on disk, do not store the plain private/secret key and do not store the plain mnemonic phrase. Instead, use `Wallet::encrypt` to encrypt its content first before saving it to disk.
