# Setup instructions for the Fuel SDK

## What you will need on your machine

- The latest `stable` Rust toolchain: <https://fuellabs.github.io/sway/latest/introduction/installation.html#dependencies>
- `forc` and `fuel-core` binaries: <https://fuellabs.github.io/sway/latest/introduction/installation.html>

`forc` is Sway equivalent of Rust's `cargo`. `fuel-core` is a Fuel full node implementation.

Now you're up and ready to develop with the Fuel Rust SDK!

## Importing the Fuel Rust SDK

Add these dependencies on your `Cargo.toml`:

```toml
fuels-abigen-macro = "0.14"
fuels = "0.14"
```

> **Note** We're using version `0.14` of the SDK, which is the latest version at the time of this writing.

And then, in your Rust file that's going to make use of the SDK:

```rust,ignore
use fuels::prelude::*;
use fuels_abigen_macro::abigen;
```
