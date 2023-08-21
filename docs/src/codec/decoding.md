# Decoding
Be sure to read the [prerequisites](./index.md#prerequisites-for-decodingencoding) to decoding.

Decoding is done via the `AbiDecoder`:

```rust,ignore
{{#include ../../../examples/codec/src/lib.rs:decoding_example}}
```
First into a `Token`, then via the `Tokenizable` trait, into the desired type.

If the type came from `abigen!` (or uses the `::fuels::macros::TryFrom` derivation) then you can also use `try_into` to convert bytes into a type that implements both `Parameterize` and `Tokenizable`:
```rust,ignore
{{#include ../../../examples/codec/src/lib.rs:decoding_example_try_into}}
```
Under the hood `try_from_bytes` is being called which basically does what the preceding example did.

## Configuring the decoder
The decoder can be configured to limit its resource expenditure:
```rust,ignore
{{#include ../../../examples/codec/src/lib.rs:configuring_the_decoder}}
```

For explanation of each configuration value visit the `DecoderConfig` docs.rs page.

The default values for the `DecoderConfig` are:
```rust,ignore
{{#include ../../../packages/fuels-core/src/codec/abi_decoder.rs:default_decoder_config}}
```

## Configuring the decoder for contract/script calls
You can also configure the decoder used to decode the return value of the contract method:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:contract_decoder_config}}
```

The same method is available for script calls.
