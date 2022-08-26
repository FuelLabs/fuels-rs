# Wallet Access

The kinds of operations we can perform with a `Wallet` instance depend on
whether or not we have access to the wallet's private key.

In order to differentiate between `Wallet` instances that know their private key
and those that do not, we use the `Unlocked` and `Locked` states respectively.

## Wallet States

An `Unlocked` wallet (i.e. a wallet of type `Wallet<Unlocked>`) is a wallet
whose private key is known and stored internally in memory. A wallet must be in
the `Unlocked` state in order to perform operations that involve [signing
transactions](./signing.md).

A `Locked` wallet (i.e. a wallet of type `Wallet<Locked>`) is a wallet whose
private key is *not* known or stored in memory. Instead, a locked wallet only
knows its public address. A locked wallet cannot be used to sign transactions,
however it may still perform a whole suite of useful operations including
listing transactions, assets, querying balances, and so on.

If a wallet's type is unspecified its state is `Locked` by default. That is, the
type `Wallet` (without providing any type arguments) is equivalent to
`Wallet<Locked>`. This default is aimed at encouraging users to prefer the
safer `Locked` state by default and to ensure that users remain conscious and
aware when holding their wallet's private key in memory.

## Transitioning States

A `Locked` wallet can be unlocked by providing the private key:

```rust,ignore
let unlocked_wallet = locked_wallet.unlock(private_key);
```

An `Unlocked` wallet can be locked using the `lock` method:

```rust,ignore
let locked_wallet = unlocked_wallet.lock();
```

Most `Wallet` constructors that create or generate a new wallet are returned in
the `Unlocked` state. Consider `lock`ing the wallet after the new private key
has been handled in order to reduce the scope in which the wallet's private key
is stored in memory.

## Design Guidelines

When designing APIs that accept a `Wallet` as an input, we should think
carefully about the kind of access that we require. API developers should aim to
minimise their usage of `Unlocked` wallets in order to ensure private keys are
stored in memory no longer than necessary in order to reduce the surface area
for attacks and vulnerabilities in downstream libraries and applications.
