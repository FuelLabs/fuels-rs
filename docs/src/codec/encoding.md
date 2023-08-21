# Encoding
<!-- TODO: replace all 0.46.0 with 'latest' -->
<!-- TODO: Replace all instances of `Token` and similar references with links to docs.rs. -->
Be sure to read the [prerequisites](./index.md#prerequisites-for-decodingencoding) to encoding.

Encoding is done via the [`ABIEncoder`](https://docs.rs/fuels/0.46.0/fuels/core/codec/struct.ABIEncoder.html):

```rust,ignore
{{#include ../../../examples/codec/src/lib.rs:encoding_example}}
```
Note that the return type of `encode` is `UnresolvedBytes`. The encoding cannot be finished until we know at which memory address this data is to be loaded on. If you don't use heap types (`::std::vec::Vec`, `::fuels::types::Bytes`, `::std::string::String`) then you can safely `.resolve(0)` to get the encoded bytes.

There is also a shortcut-macro that can encode multiple types which implement `Tokenizable`:

```rust,ignore
{{#include ../../../examples/codec/src/lib.rs:encoding_example_w_macro}}
```
> Note:
> The above example will call `.resolve(0)`. Don't use it if you're encoding heap types.
