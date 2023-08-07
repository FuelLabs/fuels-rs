# Integration tests structure in `fuels-rs`

The integration tests of `fuels-rs` cover almost all aspects of the SDK and have grown significantly as more functionality was added. To make the tests and associated `Sway` projects more manageable they were split into several categories. A category consist of a `.rs` file for the tests and, if needed, a separate directory for the `Sway` projects.

Currently have the following structure:

```shell
  .
  ├─  bindings/
  ├─  contracts/
  ├─  logs/
  ├─  predicates/
  ├─  storage/
  ├─  types/
  ├─  bindings.rs
  ├─  contracts.rs
  ├─  from_token.rs
  ├─  logs.rs
  ├─  predicates.rs
  ├─  providers.rs
  ├─  scripts.rs
  ├─  storage.rs
  ├─  types.rs
  └─  wallets.rs
```

Even though test organization is subjective, please consider these guidelines before adding a new category:

- Add a new category when creating a new section in the `Fuels Rust SDK` book - e.g. `Types`
- Add a new category if there are more than 3 test and more than 100 lines of code and they form a group of tests - e.g. `storage.rs`

 Otherwise, we recommend putting the integration test inside the existing categories above.
