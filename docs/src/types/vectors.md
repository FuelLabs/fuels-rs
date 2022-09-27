# Vectors

## Passing in vectors

You can pass a Rust `std::vec::Vec` into your contract method transparently. The following code calls a Sway function which accepts and then returns a `Vec<SomeStruct<u32>>`.

```rust,ignore
{{#include ../../../packages/fuels/tests/harness.rs:passing_in_vec}}
```

You can use a vector just like you would use any other type -- i.e. a `[Vec<u32>; 2]` or a `SomeStruct<Vec<Bits256>>` etc.

## Returning vectors

There is a mandatory extra step to returning vectors -- you need to `log` every element before returning the vector.

You must not mix any unrelated `log`s once you start logging vector elements. 

These logs must be the last ones you make.


### A simple case:

```Rust
contract;

use std::logging::log;

abi MyContract {
    fn u32_vec(arg: Vec<u32>) -> Vec<u32>;
}


impl MyContract for Contract {
    fn some_fn() -> Vec<u32> {
        let mut a_vec = ~Vec::new();
        a_vec.push(1);
        a_vec.push(2);

        log_vec(expected);
        a_vec
    }
}

fn log_vec<T>(vec: Vec<T>) {
    let mut i = 0;
    while i < vec.len() {
        log(vec.get(i).unwrap());
        i += 1;
    }
}
```

### Respect the order

If you have vectors embedded in some other type, you must take care to log them in order:

```Rust
contract;

use std::logging::log;

struct Child {
    grandchild: Vec<u32>,
    info: Vec<u32>,
}
struct Parent {
    child: Child,
    info: Vec<u32>,
}

abi MyContract {
    fn test_function() -> Parent;
}

impl MyContract for Contract {
    fn test_function() -> Parent {
        let mut grandchild_vec = ~Vec::new();
        grandchild_vec.push(0);

        let mut child_info_vec = ~Vec::new();
        child_info_vec.push(1);

        let child = Child {
            grandchild: grandchild_vec,
            info: child_info_vec,
        };

        let mut parent_info_vec = ~Vec::new();
        parent_info_vec.push(2);

        let parent = Parent {
            child,
            info: parent_info_vec,
        };

        log_vec(grandchild_vec);
        log_vec(child_info_vec);
        log_vec(parent_info_vec);

        parent
    }
}
```

### Nested vectors

There is one more step you must take if you're logging a vector nested immediately inside another vector -- i.e. `Vec<Vec<u32>>`

An example:

```Rust
contract;

use std::logging::log;

abi MyContract {
    fn test_function() -> Vec<Vec<u32>>;
}

impl MyContract for Contract {
    fn test_function() -> Vec<Vec<u32>> {
        let mut parent_vec = ~Vec::new();

        let mut inner_vec_1 = ~Vec::new();
        inner_vec_1.push(1);
        parent_vec.push(inner_vec_1);

        let mut inner_vec_2 = ~Vec::new();
        inner_vec_2.push(2);
        parent_vec.push(inner_vec_2);

        log_vec(inner_vec_1);
        log_vec(inner_vec_2);
        log_vec(parent_vec);

        parent_vec
    }
}
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
