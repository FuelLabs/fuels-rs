# Integration tests structure in `fuels-rs`

The integration tests of `fuels-rs` cover almost all aspects of the SDK and have grown significantly as more functionality is added. To make the test more manageable, they were split into several `modules` together with the corresponding `Sway` projects. The modules are defined in the `packages/fuels/test/harness.rs` file and currently have the following structure:

```rust,ignore
{{#include ../../../packages/fuels/tests/harness.rs:test_modules}}
```

Even though adding new test modules is subjective, please consider these guidelines when adding new ones:
 - Add a new module when creating a new section in the `Fuels Rust SDK` book - e.g. `Types`
 - Add a new module if there are more than 3 test and more than 100 lines of code and they form a group of tests - e.g. `mod storage`
