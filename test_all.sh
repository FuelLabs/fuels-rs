#!/bin/bash -e
script_dir="$(dirname "$(readlink --canonicalize-existing "$0")")"

cd "$script_dir"

# No sense in testing e2e if our own tests indicate problems
cargo test --all --all-features

exec "$script_dir/test_e2e.sh" "$@"
