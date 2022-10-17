# Integration tests structure in `fuels-rs`

The integration tests of `fuels-rs` cover almost all aspects of the SDK and have grown significantly as more functionality was added. To make the test more manageable, they were split into several `modules` together with the corresponding `Sway` projects. The modules are defined in the `packages/fuels/test/harness.rs` file and currently have the following structure:

```rust,ignore
{{#include ../../../packages/fuels/tests/harness.rs:test_modules}}
```

Even though test organization is subjective, please consider these guidelines before adding a new category (`module`):
 - Add a new module when creating a new section in the `Fuels Rust SDK` book - e.g. `Types`
 - Add a new module if there are more than 3 test and more than 100 lines of code and they form a group of tests - e.g. `mod storage`

 Otherwise, we recommend putting the integration test inside the existing modules above.
