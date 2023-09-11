# Decoding

Be sure to read the [prerequisites](./index.md#prerequisites-for-decodingencoding) to decoding.

Decoding is done via the [`ABIDecoder`](https://docs.rs/fuels/latest/fuels/core/codec/struct.ABIDecoder.html):

```rust,ignore
{{#include ../../../examples/codec/src/lib.rs:decoding_example}}
```

First into a [`Token`](https://docs.rs/fuels/latest/fuels/types/enum.Token.html), then via the [`Tokenizable`](https://docs.rs/fuels/latest/fuels/core/traits/trait.Tokenizable.html) trait, into the desired type.

If the type came from [`abigen!`](../abigen/index.md) (or uses the [`::fuels::macros::TryFrom`](https://docs.rs/fuels/latest/fuels/macros/derive.TryFrom.html) derivation) then you can also use `try_into` to convert bytes into a type that implements both [`Parameterize`](https://docs.rs/fuels/latest/fuels/core/traits/trait.Parameterize.html) and [`Tokenizable`](https://docs.rs/fuels/latest/fuels/core/traits/trait.Tokenizable.html):

```rust,ignore
{{#include ../../../examples/codec/src/lib.rs:decoding_example_try_into}}
```

Under the hood, [`try_from_bytes`](https://docs.rs/fuels/latest/fuels/core/codec/fn.try_from_bytes.html) is being called, which does what the preceding example did.

## Configuring the decoder

The decoder can be configured to limit its resource expenditure:

```rust,ignore
{{#include ../../../examples/codec/src/lib.rs:configuring_the_decoder}}
```

<!-- TODO: Add a link once a release is made -->
<!-- https://docs.rs/fuels/latest/fuels/core/codec/struct.DecoderConfig.html -->
For an explanation of each configuration value visit the `DecoderConfig`.

<!-- TODO: add a link once a release is made -->
<!-- https://docs.rs/fuels/latest/fuels/core/codec/struct.DecoderConfig.html -->
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
