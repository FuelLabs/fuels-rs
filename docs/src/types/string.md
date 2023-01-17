# String

The Rust SDK represents Fuel's `String`s as `SizedAsciiString<LEN>`, where the generic parameter `LEN` is the length of a given string. This abstraction is necessary because all strings in Fuel and Sway are statically-sized, i.e., you must know the size of the string beforehand.

Here's how you can create a simple string using `SizedAsciiString`:

```rust,ignore
{{#include ../../../packages/fuels-types/src/core/sized_ascii_string.rs:string_simple_example}}
```

To make working with `SizedAsciiString`s easier, you can use `try_into()` to convert from Rust's `String` to `SizedAsciiString`, and you can use `into()` to convert from `SizedAsciiString` to Rust's `String`. Here are a few examples:

```rust,ignore
{{#include ../../../packages/fuels-types/src/core/sized_ascii_string.rs:conversion}}
```

If your contract's method takes and returns, for instance, a Sway's `str[23]`. When using the SDK, this method will take and return a `SizedAsciiString<23>`, and you can pass a string to it like this:

```rust,ignore
{{#include ../../../packages/fuels/tests/bindings.rs:contract_takes_string}}
```
