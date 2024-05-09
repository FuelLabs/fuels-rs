# Encoding

Be sure to read the [prerequisites](./index.md#prerequisites-for-decodingencoding) to encoding.

Encoding is done via the [`ABIEncoder`](https://docs.rs/fuels/latest/fuels/core/codec/struct.ABIEncoder.html):

```rust,ignore
{{#include ../../../examples/codec/src/lib.rs:encoding_example}}
```

There is also a shortcut-macro that can encode multiple types which implement [`Tokenizable`](https://docs.rs/fuels/latest/fuels/core/traits/trait.Tokenizable.html):

```rust,ignore
{{#include ../../../examples/codec/src/lib.rs:encoding_example_w_macro}}
```

## Configuring the encoder

The encoder can be configured to limit its resource expenditure:

```rust,ignore
{{#include ../../../examples/codec/src/lib.rs:configuring_the_encoder}}
```

The default values for the `EncoderConfig` are:

```rust,ignore
{{#include ../../../packages/fuels-core/src/codec/abi_encoder.rs:default_encoder_config}}
```

## Configuring the encoder for contract/script calls

You can also configure the encoder used to encode the arguments of the contract method:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:contract_encoder_config}}
```

The same method is available for script calls.
