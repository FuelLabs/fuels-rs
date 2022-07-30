# Setup instructions for the Fuel SDK

These are the steps you need to use the Fuel Rust SDK.

## Dependencies

- [The latest `stable` Rust toolchain](https://fuellabs.github.io/sway/master/introduction/installation.html);
- [`forc` and `fuel-core` binaries](https://fuellabs.github.io/sway/master/introduction/installation.html#installing-from-cargo).

`forc` is Sway equivalent of Rust's `cargo`. `fuel-core` is a Fuel full node implementation.

Now you're up and ready to develop with the Fuel Rust SDK!

## Importing the Fuel Rust SDK

Add these dependencies on your `Cargo.toml`:

```toml
fuels = "0.19"
```

> **Note** We're using version `0.19` of the SDK, which is the latest version at the time of this writing.

And then, in your Rust file that's going to make use of the SDK:

```rust,ignore
use fuels::prelude::*;
```

## More in-depth Fuel and Sway knowledge

Read [The Sway Book](https://fuellabs.github.io/sway/master/introduction/overview.html) for more in-depth knowledge about Sway, the official smart contract language for the Fuel Virtual Machine.
