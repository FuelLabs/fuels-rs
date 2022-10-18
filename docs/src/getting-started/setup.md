# Setting up and running the Fuel Rust SDK

## Dependencies

- [The latest `stable` Rust toolchain](https://fuellabs.github.io/sway/master/introduction/installation.html);
- [`forc` and `fuel-core` binaries](https://fuellabs.github.io/sway/master/introduction/installation.html#installing-from-cargo).

`forc` is Sway equivalent of Rust's `cargo`. `fuel-core` is a Fuel full node implementation.

There are two main ways you can use the Fuel Rust SDK:
1. Creating a new Sway project with `forc` and running the tests
2. Creating a standalone project and importing the `fuels-rs` crate

## Creating a new project with Forc

You can create a new Sway project with

```
forc new <Project name>
```

Or you can initialize a project within an existing folder with

```
forc init
```

`forc` will setup an example project and we can test it with

```
forc test
```

> **Note** If you need to capture output from the tests, use one of the following commands:

```
forc test -- --nocapture
```
```
RUST_LOG=receipts cargo test --test integration_tests
```

## Importing the Fuel Rust SDK

Add these dependencies on your `Cargo.toml`:

```toml
fuels = "0.26"
```

> **Note** We're using version `0.26` of the SDK, which is the latest version at the time of this writing.

And then, in your Rust file that's going to make use of the SDK:

```rust,ignore
use fuels::prelude::*;
```

## The Fuel Rust SDK source code

Another way to experience the SDK is to look at the source code. The `packages/fuels/tests/` folder is full of integration tests that go through almost all aspects of the SDK.

> **Note** Before running the tests, we need to build all the Sway test projects. The SDK has a binary that will go through all projects and build them for us. You can use it with the following command.

```
cargo run --bin build-test-projects
```

Then we can run the tests with

```
cargo test
```

If you need all targets and all features, you can run

```
cargo test --all-targets --all-features
```

> **Note** If you need to capture output from the tests, you can run

```
cargo test -- --nocapture
```

## More in-depth Fuel and Sway knowledge

Read [The Sway Book](https://fuellabs.github.io/sway/master/introduction/overview.html) for more in-depth knowledge about Sway, the official smart contract language for the Fuel Virtual Machine.
