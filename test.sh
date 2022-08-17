#!/bin/bash -e
script_dir="$(dirname "$(readlink --canonicalize-existing "$0")")"

cd "$script_dir"

# No sense in testing e2e if our own tests indicate problems
cargo test --all --all-features

git submodule update fuel-e2e-tests

cd "fuel-e2e-tests"

original_cargo_toml="$(mktemp --suffix ".toml")"
mv Cargo.toml "$original_cargo_toml"

revert_cargo_toml(){
	mv "$original_cargo_toml" Cargo.toml
}

trap revert_cargo_toml EXIT

cat "$original_cargo_toml" "$script_dir/.e2e_patch.toml" > Cargo.toml

cargo xtask test
