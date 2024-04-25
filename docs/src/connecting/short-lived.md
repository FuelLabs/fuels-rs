# Running a short-lived Fuel node with the SDK

You can use the SDK to spin up a local, ideally short-lived Fuel node. Then, you can instantiate a Fuel client, pointing to this node.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:instantiate_client}}
```

This approach is ideal for contract testing.

You can also use the test helper `setup_test_provider()` for this:

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:create_random_wallet}}
```

You can also use `launch_provider_and_get_wallet()`, which abstracts away the `setup_test_provider()` and the wallet creation, all in one single method:

```rust,ignore
let wallet = launch_provider_and_get_wallet().await?;
```

## Features

### Fuel-core lib

The `fuel-core-lib` feature allows us to run a `fuel-core` node without installing the `fuel-core` binary on the local machine. Using the `fuel-core-lib` feature flag entails downloading all the dependencies needed to run the fuel-core node.

```rust,ignore
fuels = { version = "0.58.0", features = ["fuel-core-lib"] }
```

### RocksDB

The `rocksdb` is an additional feature that, when combined with `fuel-core-lib`, provides persistent storage capabilities while using `fuel-core` as a library.

```rust,ignore
fuels = { version = "0.58.0", features = ["rocksdb"] }
```
