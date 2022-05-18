# Managing wallets

Wallets are used for many important things, for instance:

1. Checking your balance;
2. Transferring coins to a destination address;
3. Signing messages and transactions;
4. Paying for network fees when sending transactions or deploying smart contracts.

The SDK gives you many different ways to create wallets. Let's explore these different ways.

## Creating a wallet from a Private Key

You can use `fuel_crypto` to generate a random `SecretKey` and use it to create a wallet from it:

```Rust
use fuel_crypto::{SecretKey};

// Generate a random private/secret key.
let mut rng = rand::thread_rng();
let secret = SecretKey::random(&mut rng);

// Use the test helper to setup a test provider.
let (provider, _address) = setup_test_provider(vec![]).await;

// Create the wallet.
let wallet = LocalWallet::new_from_private_key(secret, provider);
```

## Creating a wallet from a mnemonic phrase

A mnemonic phrase is a cryptographically-generated sequence of words that's used to derive a private key. For instance: `"oblige salon price punch saddle immune slogan rare snap desert retire surprise";` would generate the address `0xdf9d0e6c6c5f5da6e82e5e1a77974af6642bdb450a10c43f0c6910a212600185`. 

In addition to that, we also support [Hierarchical Deterministic Wallets](https://www.ledger.com/academy/crypto/what-are-hierarchical-deterministic-hd-wallets) and [derivation paths](https://learnmeabitcoin.com/technical/derivation-paths). You may recognize the string `"m/44'/60'/0'/0/0"` from somewhere; that's a derivation path. In simple terms, it's a way to derive many wallets from a single root wallet.

The SDK gives you two wallet from mnemonic instantiation methods: one that takes a derivation path (`Wallet::new_from_mnemonic_phrase_with_path`) and one that uses the default derivation path, in case you don't want or don't need to configure that (`Wallet::new_from_mnemonic_phrase`).

Here's how you can create wallets with both mnemonic phrases and derivation paths:

```Rust
let phrase = "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

// Use the test helper to setup a test provider.
let (provider, _address) = setup_test_provider(vec![]).await;

// Create first account from mnemonic phrase.
let wallet =
    Wallet::new_from_mnemonic_phrase_with_path(phrase, provider, "m/44'/60'/0'/0/0")
        .unwrap();

// Or with the default derivation path
let wallet = Wallet::new_from_mnemonic_phrase(phrase, provider).unwrap();

let expected_address = "df9d0e6c6c5f5da6e82e5e1a77974af6642bdb450a10c43f0c6910a212600185";

assert_eq!(wallet.address().to_string(), expected_address);
```

## Creating a wallet and storing an encrypted JSON wallet to disk

You can also manage a wallet using [JSON wallets](https://cryptobook.nakov.com/symmetric-key-ciphers/ethereum-wallet-encryption) -- wallets that are securely encrypted and stored on disk. This makes it easier to manage multiple wallets, especially for testing purposes.

You can create a random wallet and, at the same time, encrypto and store it:

```Rust
let dir = tempdir().unwrap();
let mut rng = rand::thread_rng();

// Use the test helper to setup a test provider.
let (provider, _address) = setup_test_provider(vec![]).await;

let password = "my_master_password";

// Create a wallet to be stored in the keystore.
let (wallet, uuid) =
    Wallet::new_from_keystore(&dir, &mut rng, password, provider.clone()).unwrap();
```

Then, later, you can recover the wallet if you know the master password:

```Rust
// Use the test helper to setup a test provider.
let (provider, _address) = setup_test_provider(vec![]).await;

let path = Path::new(dir.path()).join(uuid);
let password = "my_master_password";

let recovered_wallet = Wallet::load_keystore(&path, password, provider).unwrap();
```

## Encrypting and storing a wallet created from mnemonic or private key

If you had already created a wallet using a mnemonic phrase or a private key, you can also encrypt it and save it to disk:

```Rust
let dir = tempdir().unwrap();

let phrase =
    "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

// Use the test helper to setup a test provider.
let (provider, _address) = setup_test_provider(vec![]).await;

// Create first account from mnemonic phrase.
let wallet = Wallet::new_from_mnemonic_phrase(phrase, provider).unwrap();

let password = "my_master_password";

// Encrypts and stores it on disk. Can be recovered using `Wallet::load_keystore`.
let uuid = wallet.encrypt(&dir, password).unwrap();
```

## Security

Keep in mind that you should never share your private/secret key. And in the case of wallets that were derived from a mnemonic phrase, never share your mnemonic phrase.

If you're planning on storing the wallet on disk, do not store the plain private/secret key and do not store the plain mnemonic phrase. Instead, use `Wallet::encrypt` to encrypt its content first before saving it to disk. 