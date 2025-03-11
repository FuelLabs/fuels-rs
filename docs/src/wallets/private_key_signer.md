# Using private keys to create wallets

## Directly from a private key

An example of how to create a wallet that uses a private key signer:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:create_wallet_from_secret_key}}
```

There is also a helper for generating a wallet with a random private key signer:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:create_random_wallet}}
```

## From a mnemonic phrase

A mnemonic phrase is a cryptographically generated sequence of words used to create a master seed. This master seed, when combined with a [derivation path](https://thebitcoinmanual.com/articles/btc-derivation-path/), enables the generation of one or more specific private keys. The derivation path acts as a roadmap within the [hierarchical deterministic (HD) wallet structure](https://www.ledger.com/academy/crypto/what-are-hierarchical-deterministic-hd-wallets), determining which branch of the key tree produces the desired private key.

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:create_wallet_from_mnemonic}}
```

## Security Best Practices

- **Never Share Sensitive Information:**
  Do not disclose your private key or mnemonic phrase to anyone.

- **Secure Storage:**
  When storing keys on disk, **always encrypt** them (the SDK provides a [`Keystore`](./keystore.md). This applies to both plain private/secret keys and mnemonic phrases.
