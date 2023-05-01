#!/usr/bin/env bash

# Requires installed:
# The latest version of the `forc`,`forc-fmt` and `fuel-core`.
# `cargo install fuel-core-bin --git https://github.com/FuelLabs/fuel-core --tag v0.17.11 --locked`
# `cargo install forc --git https://github.com/FuelLabs/sway --tag v0.38.0 --locked`
# `cargo install forc-fmt --git https://github.com/FuelLabs/sway --tag v0.38.0 --locked`
# Note, if you need a custom branch, you can replace `--tag {RELEASE}` with the `--branch {BRANCH_NAME}`.

cargo fmt --all -- --check &&
forc fmt --check --path packages/fuels &&
forc build --terse --path packages/fuels &&
forc build --terse --json-abi-with-callpaths --path packages/fuels &&
cargo clippy --all-targets &&
cargo clippy --all-targets --all-features &&
cargo test --all-targets --all-features &&
cargo test --all-targets --all-features --workspace &&
cargo test --all-targets --workspace &&
cargo run --bin check-docs &&
cargo doc |& grep -A 6  "warning: unresolved link to"
