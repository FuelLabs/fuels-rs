# Bech32Address

`Bech32Address` enables the use of addresses in the bech32 format. It can be converted to an `Address` because they both hold the same data (public-key hash), only with different encodings.


These are the main ways of creating a `Bech32Address`: 

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:bech32}}
```
