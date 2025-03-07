# Encrypting and storing keys

## Creating a wallet and storing an encrypted JSON key on disk

You can also manage a key using [JSON keys](https://cryptobook.nakov.com/symmetric-key-ciphers/ethereum-wallet-encryption) that are securely encrypted and stored on the disk. This makes it easier to manage multiple wallets, especially for testing purposes.

You can create a random key and, at the same time, encrypt and store it. Then, later, you can recover the key if you know the master password:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:create_and_restore_json_key}}
```

## Encrypting and storing a key created from a mnemonic or private key

If you have already created a key using a mnemonic phrase or a private key, you can also encrypt it and save it to disk:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:create_and_store_mnemonic_key}}
```
