# fuels-rs

[![build](https://github.com/FuelLabs/fuels-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuels-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fuels?label=latest)](https://crates.io/crates/fuels)
[![docs](https://docs.rs/fuels/badge.svg)](https://docs.rs/fuels)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

Rust SDK for Fuel. It can be used for a variety of things, including but not limited to:

- Compiling, deploying, and testing [Sway](https://github.com/FuelLabs/sway) contracts;
- Launching a local Fuel network;
- Crafting and signing transactions with hand-crafted scripts or contract calls;
- Generating type-safe Rust bindings of contract methods;
- And more, `fuels-rs` is still in active development.

## Documentation

See [the `fuels-rs` book](https://fuellabs.github.io/fuels-rs/latest/)

## Features

- [x] Launch Fuel nodes
- [x] Deploy contracts
- [x] Interact with deployed contracts
- [x] Type-safe Sway contracts bindings code generation
- [x] Run Sway scripts
- [x] CLI for common operations
- [x] Local test wallets
- [ ] Wallet integration
- [ ] Events querying/monitoring

## FAQ

### What dependencies do I need?

- [The latest `stable` Rust toolchain](https://docs.fuel.network/guides/installation/#installing-rust);
- [`forc` and `fuel-core` binaries](https://docs.fuel.network/guides/installation/#installing-the-fuel-toolchain-using-fuelup).

### How can I run the SDK tests?

First, build the test projects using `forc`:

```shell
forc build --release --path e2e
```

Then you can run the SDK tests with:

```shell
cargo test
```

You can also run specific tests. The following example will run all integration tests in `types.rs` whose names contain `in_vector` and show their outputs:

```shell
cargo test --test types in_vector -- --show-output
```

### How to run WASM tests?

You need to have wasm32 as a target, if you don't already:

```shell
 rustup target add wasm32-unknown-unknown
```

You also need `wasm-pack`, if you don't already:

```shell
cargo install wasm-pack
```

Navigate to `packages/wasm-tests` and run `wasm-pack test`.

### What to do if my tests are failing on `master`

Before doing anything else, try all these commands:

```shell
cargo clean
rm Cargo.lock
forc build --release --path e2e
cargo test
```

### Why is the prefix `fuels` and not `fuel`?

In order to make the SDK for Fuel feel familiar with those coming from the [ethers.js](https://github.com/ethers-io/ethers.js) ecosystem, this project opted for an `s` at the end. The `fuels-*` family of SDKs is inspired by The Ethers Project.

### How can I run the docs locally?

Install `mdbook` by running:

```shell
cargo install mdbook
```

Next, navigate to the `docs` folder and run the command below to start a local server and open a new tab in you browser.

```shell
mdbook serve --open
```

You can build the book by running:

```shell
mdbook build
```
