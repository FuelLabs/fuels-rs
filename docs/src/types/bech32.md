# `Bech32`

`Bech32Address` and `Bech32ContractId` enable the use of addresses and contract IDs in the `bech32` format. They can easily be converted to their counterparts `Address` and `ContractId`.

Here are the main ways of creating a `Bech32Address`, but note that the same applies to `Bech32ContractId`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:bech32}}
```

> **Note:** when creating a `Bech32Address` from `Address` or `Bech32ContractId` from `ContractId` the `HRP` (Human-Readable Part) is set to **"fuel"** per default.
