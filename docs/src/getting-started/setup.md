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
