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

A `Wallet<S>` is a struct parameterized by a signer `S` that implements the `Signer` trait. In this setup:

- The **signer** defines how messages and transactions are actually signed (for example, using a local private key, AWS KMS, or another mechanism).
- The **wallet** holds that signer and provides a common interface for actions like sending transactions, paying fees, or querying balances.

By picking a particular signer, you decide whether the wallet can produce signatures or merely operate in a read-only capacity.
