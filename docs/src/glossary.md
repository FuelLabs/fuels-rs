# Glossary

## Contract

<!-- This section should define a contract -->
<!-- rs_contract:example:start -->
A contract, in the SDK, is an abstraction that represents a connection to a specific smart contract deployed on the Fuel Network. This contract instance can be used as a regular Rust object, with methods attached to it that reflect those in its smart contract equivalent.
<!-- rs_contract:example:end -->

## Provider

<!-- This section should define a provider -->
<!-- rs_provider:example:start -->
A Provider is a struct that provides an abstraction for a connection to a Fuel node. It provides read-only access to the node. You can use this provider as-is or through the wallet.
<!-- rs_provider:example:end -->

## Wallet and signer

<!-- This section should define a wallet and signer -->
<!-- rs_wallet_signer:example:start -->
A `Wallet` is a struct with direct or indirect access to a private key. You can use a `Wallet` to sign messages and transactions to authorize the network to charge your account to perform operations. The terms wallet and signer in the SDK are often used interchangeably, but, technically, a `Signer` is simply a Rust trait to enable the signing of transactions and messages; the `Wallet` implements the `Signer` trait.
<!-- rs_wallet_signer:example:end -->
