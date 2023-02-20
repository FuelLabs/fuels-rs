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

- [The latest `stable` Rust toolchain](https://fuellabs.github.io/sway/master/introduction/installation.html);
- [`forc` and `fuel-core` binaries](https://fuellabs.github.io/sway/master/introduction/installation.html#installing-from-cargo).

### How can I run the SDK tests?

First, build the test projects using `forc`:

```shell
forc build --path packages/fuels
```

Then you can run the SDK tests with:

```shell
cargo test
```

You can also run specific tests. The following example will run all integration tests in `types.rs` whose names contain `in_vector` and show their outputs:

```shell
cargo test --test types in_vector -- --show-output
``` 

### What to do if my tests are failing on `master`

Before doing anything else, try all these commands:

```shell
cargo clean
rm Cargo.lock
forc build --path packages/fuels
cargo test
```

### Why is the prefix `fuels` and not `fuel`?

In order to make the SDK for Fuel feel familiar with those coming from the [ethers.js](https://github.com/ethers-io/ethers.js) ecosystem, this project opted for an `s` at the end. The `fuels-*` family of SDKs is inspired by The Ethers Project.

### How can I run the docs locally?

Install `mdbook` by running:

```shell
cargo install mdbook
```

Next, navigate to the `docs` folder and run the command below to start a local server.

```shell
mdbook serve
```

To view the docs, navigate to the localhost address output by `mdbook`, which is http://localhost:3000/ by default.

You can build the book by running:

```shell
mdbook build
```

