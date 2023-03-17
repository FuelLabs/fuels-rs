# Vectors

## Passing in vectors

You can pass a Rust `std::vec::Vec` into your contract method transparently. The following code calls a Sway contract method which accepts a `Vec<SomeStruct<u32>>`.

```rust,ignore
{{#include ../../../packages/fuels/tests/contract_types.rs:passing_in_vec}}
```

You can use a vector just like you would use any other type -- e.g. a `[Vec<u32>; 2]` or a `SomeStruct<Vec<Bits256>>` etc.

## Returning vectors

Returning vectors from contract methods is supported transparently, with the caveat that you cannot have them nested inside another type.

Which means you can return for instance:
```rust,ignore
Vec<u32>
Vec<SomeStruct>
Vec<Bits256>
```

but not return this kind of struct:
```rust,ignore
InvalidStruct {
    bim: Vec<u32>,
    bam: u64
}
```

** >Note: you can still interact with contracts containing methods that return such structs, just not interact with the methods themselves ** 
