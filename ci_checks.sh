#!/usr/bin/env bash

# Requires installed:
# The latest version of the `forc`,`forc-fmt` and `fuel-core`.
# `cargo install fuel-core-bin --git https://github.com/FuelLabs/fuel-core --tag v0.16.1 --locked`
# `cargo install forc --git https://github.com/FuelLabs/sway --tag v0.35.0 --locked`
# `cargo install forc-fmt --git https://github.com/FuelLabs/sway --tag v0.35.0 --locked`
# Note, if you need a custom branch, you can replace `--tag {RELEASE}` with the `--branch {BRANCH_NAME}`.

cargo run --bin test-projects -- build &&
cargo run --bin test-projects -- format --check &&
cargo fmt --all --verbose -- --check &&
cargo clippy --all-targets --all-features &&
cargo test --all-targets --all-features &&
cargo test --all-targets &&
cargo test --all-targets &&
# May fail after `cargo doc`
cargo run --bin check-docs
