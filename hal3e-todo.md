- [x] transaction: `fuel-tx`
  - [x] add `Unknown` transaction variant
  - [x] add `Unknown` `TransactionRepr`
  - [x] refactor `fuel-core` and `fuel-core-client`
  - [x] refactor `fuels-rs`

- [x] coin-type: `fuels-rs`
  - [x] `fuel-core-client` already has an `Unknown` variant - update the `fuels-rs`

- [ ] consensus-parameters: `fuel-tx` and `fuel-core-client`
  - [ ] update conversion from schema type to the `fuel-tx` type
  - [ ] problems getters. What should we return `Result` or `Option`?
  - [ ] does the cparams version change if we change for example the `FeeParameters` to another version


- [x] block-headers - could not do anything as the types is a struct
- [ ] opcodes - INVESTIGATE
