# Setup instructions for the Fuel SDK

## What you will need on your machine

- Rust 2021 ([see here](https://doc.rust-lang.org/cargo/getting-started/installation.html))
- A clone of the `fuels-rs` repository:

```sh
git clone https://github.com/FuelLabs/fuels-rs
```

- Install the following `cargo` crates:

```sh
cargo install forc fuel-core
```

- `forc` is the crate that holds the Sway language and Fuel's equivalent of `cargo`
- `fuel-core` is the crate that contains the Fuel node software and execution

Now you're up and ready to develop with the Fuel Rust SDK!

## Importing the Fuel Rust SDK

all you need is to declare these three dependencies on your `Cargo.toml`:

```toml
fuel-tx = "0.10"
fuels-abigen-macro = "0.12"
fuels = "0.12"
```

_Note that we're using `0.12`, which is the latest version at the time of this writing._

And then, in your Rust file that's going to make use of the SDK:

```Rust
use fuels::prelude::*;
use fuels_abigen_macro::abigen;
```
