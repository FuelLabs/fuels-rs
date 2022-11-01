# Vectors

## Passing in vectors

You can pass a Rust `std::vec::Vec` into your contract method transparently. The following code calls a Sway contract method which accepts a `Vec<SomeStruct<u32>>`.

```rust,ignore
{{#include ../../../packages/fuels/tests/types.rs:passing_in_vec}}
```

You can use a vector just like you would use any other type -- e.g. a `[Vec<u32>; 2]` or a `SomeStruct<Vec<Bits256>>` etc.

## Returning vectors

This is currently not supported. If you try returning a type that is or contains a vector you will get a compile time error.
