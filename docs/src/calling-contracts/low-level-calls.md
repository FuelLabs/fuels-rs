# Low-level calls

<!-- This section should explain what low-level calls are and how to do them -->
With low-level calls, you can specify the parameters of your calls at runtime and make indirect calls through other contracts.

Your caller contract should call `std::low_level_call::call_with_function_selector`, providing:

- target contract ID
- function selector encoded as `Bytes`
- calldata encoded as `Bytes`
- whether the calldata contains only a single value argument (e.g. a `u64`)
- `std::low_level_call::CallParams`

```rust,ignore
{{#include ../../../packages/fuels/tests/contracts/low_level_caller/src/main.sw:low_level_call_contract}}
```

On the SDK side, you can construct an encoded function selector using the `fuels::core::fn_selector!` macro, and encoded calldata using the `fuels::core::calldata!` macro.

E.g. to call the following function on the target contract:

```rust,ignore
{{#include ../../../packages/fuels/tests/contracts/contract_test/src/main.sw:low_level_call}}
```

you would construct the function selector and the calldata as such, and provide them to the caller contract (like the one above):

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:low_level_call}}
```
