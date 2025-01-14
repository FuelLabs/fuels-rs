# Getting Started

## Installation Guide

Please visit the Fuel [installation guide](https://docs.fuel.network/guides/installation) to install the Fuel toolchain binaries and prerequisites.

`forc` is Sway equivalent of Rust's `cargo`. `fuel-core` is a Fuel full node implementation.

There are two main ways you can use the Fuel Rust SDK:

1. Creating a new Sway project with `forc` and running the tests
2. Creating a standalone project and importing the `fuels-rs` crate

## Creating a new project with Forc

You can create a new Sway project with

```shell
forc new <Project name>
```

Or you can initialize a project within an existing folder with

```shell
forc init
```

### Adding a Rust integration test to the Sway project

Now that we have a new project, we can add a Rust integration test using a `cargo generate` template.
If `cargo generate` is not already installed, you can install it with:

<!-- This section should have the command to install cargo generate -->
<!-- cargo_gen_install:example:start -->
```shell
cargo install cargo-generate
```
<!-- cargo_gen_install:example:end -->

> **Note** You can learn more about cargo generate by visiting its [repository](https://github.com/cargo-generate/cargo-generate).

Let's generate the default test harness with the following command:

<!-- This section should have the command to cargo generate a test harness -->
<!-- cargo_gen:example:start -->
```shell
cargo generate --init fuellabs/sway templates/sway-test-rs --name <Project name> --force
```
<!-- cargo_gen:example:end -->

<!-- This section should explain the `--force` flag -->
<!-- force_flag:example:start -->
`--force` forces your `--name` input to retain your desired casing for the `{{project-name}}` placeholder in the template. Otherwise, `cargo-generate` automatically converts it to kebab-case. With `--force`, this means that both `my_fuel_project` and `my-fuel-project` are valid project names, depending on your needs.
<!-- force_flag:example:end -->

Before running test, we need to build the Sway project with:

```shell
forc build
```

Afterwards, we can run the test with:

<!-- This section should have the command to run a test -->
<!-- run_test:example:start -->
```shell
cargo test
```
<!-- run_test:example:end -->

> **Note** If you need to capture output from the tests, use one of the following commands:

<!-- This section should have the command to run a test with no capture -->
<!-- run_test_nocap:example:start -->
```shell
cargo test -- --nocapture
```
<!-- run_test_nocap:example:end -->

## Importing the Fuel Rust SDK

Add these dependencies on your `Cargo.toml`:

```toml
fuels = "0.66.0"
```

> **Note** We're using version `0.66.0` of the SDK, which is the latest version at the time of this writing.

And then, in your Rust file that's going to make use of the SDK:

```rust,ignore
use fuels::prelude::*;
```

## The Fuel Rust SDK source code

Another way to experience the SDK is to look at the source code. The `e2e/tests/` folder is full of integration tests that go through almost all aspects of the SDK.

> **Note** Before running the tests, we need to build all the Sway test projects. The file `packages/fuels/Forc.toml` contains a `[workspace], which members are the paths to all integration tests.
> To build these tests, run the following command:

```shell
forc build --release --path e2e
```

> `forc` can also be used to clean and format the test projects. Check the `help` output for more info.

After building the projects, we can run the tests with

```shell
cargo test
```

If you need all targets and all features, you can run

```shell
cargo test --all-targets --all-features
```

> **Note** If you need to capture output from the tests, you can run

```shell
cargo test -- --nocapture
```

## More in-depth Fuel and Sway knowledge

Read [The Sway Book](https://docs.fuel.network/docs/sway/) for more in-depth knowledge about Sway, the official smart contract language for the Fuel Virtual Machine.
