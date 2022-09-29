# Vectors

## Passing in vectors
You can pass a Rust std::vec::Vec into your contract function transparently. The following code calls a sway function which accepts and then returns a `Vec<SomeStruct<u32>>`.

```rust,ignore
{{#include ../../../packages/fuels/tests/harness.rs:passing_in_vec}}
```

You can use a vector just like you would use any other type -- i.e. a `[Vec<u32>; 2]` or a `SomeStruct<Vec<Bits256>>` etc.

## Returning vectors
There is a mandatory extra step to returning vectors -- you need to `log` every element before returning the vector.

You must not mix any unrelated `log`s once you start logging vector elements. 

These logs must be the last ones you make.


### A simple case:
```rust,ignore
{{#include ../../../packages/fuels/tests/test_projects/vectors/src/main.sw:sway_returning_a_vec}}
```

Calling it from the SDK would look like this:

```rust,ignore
{{#include ../../../packages/fuels/tests/harness.rs:harness_returning_a_vec}}
```

### Respect the order
If you have vectors embedded in some other type, for example:

```rust,ignore
{{#include ../../../packages/fuels/tests/test_projects/vectors/src/data_structures.sw:sway_nested_vec_types}}
```

you must take care to log them in order:
```rust,ignore
{{#include ../../../packages/fuels/tests/test_projects/vectors/src/main.sw:sway_returning_type_w_nested_vectors}}
```

where `log_vec` is a helper defined as:
```rust,ignore
{{#include ../../../packages/fuels/tests/test_projects/vectors/src/utils.sw:sway_log_vec_helper}}
```


Calling it from the SDK would look like this:

```rust,ignore
{{#include ../../../packages/fuels/tests/harness.rs:harness_returning_type_w_nested_vectors}}
```


### Nested vectors
There is one more step you must take if you're logging a vector nested immediately inside another vector -- e.g. `Vec<Vec<u32>>`

An example:

```rust,ignore
{{#include ../../../packages/fuels/tests/test_projects/vectors/src/main.sw:sway_returning_immediately_nested_vectors}}
```
Calling it from the SDK would look like this:

```rust,ignore
{{#include ../../../packages/fuels/tests/harness.rs:harness_returning_immediately_nested_vectors}}
```


To log a `Vec<Vec<u32>>` we need to do the following:

Step 1. 
We're logging the parent vector. The type is `Vec<Vec<..>>`.

This is a vector with two elements `inner_vec_1` and `inner_vec_2`. Both are vectors themselves. Log the elements of `inner_vec_1`.

Step 2.
We're logging `inner_vec_1`. The type is `Vec<u32>`.

This is a vector with one element: `1`. Log it.

The logging of `inner_vec_1` is finished.

Step 3.
We're logging the second element of the parent vector `inner_vec_2`. The type is `Vec<u32>`.

This is a vector with one element: `2`. Log it.

The logging of `inner_vec_2` is finished.

Step 4.
We've finished logging the elements of the parent vector. Since the type is `Vec<Vec<..>>` we have an additional step -- call `log` on each element of the vector:


`log(inner_vec_1)`

`log(inner_vec_2)`

In the example this is done by the `log_vec` helper.