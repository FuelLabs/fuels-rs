# Testing Basics

If you're new to Rust, you'll want to review these important tools to help you build tests.

## The `assert!` macro

You can use the `assert!` macro to enforce certain outcomes in your test. This macro invokes `panic!()` and fails the test if the expression inside evaluates to `false`.

```rust, ignore
assert!(value == 5);
```

## The `assert_eq!` macro

The `assert_eq!` macro works a lot like the `assert` macro, however instead it accepts two values, and throws and error if those values are not equal.

```rust, ignore
assert_eq!(balance, 100);
```

## The `assert_ne!` macro

The `assert_ne!` macro works just like the `assert_eq!` macro, but it will throw an error if the two values are equal.

```rust, ignore
assert_ne!(address, 0);
```

## The `println!` macro

You can use the `println!` macro to print values to the console.

```rust, ignore
println!("WALLET 1 ADDRESS {}", wallet_1.address());
println!("WALLET 1 ADDRESS {:?}", wallet_1.address());
```

Using `{}` will print the value, and using `{:?}` will print the value plus its type.

Using `{:?}` will also allow you to print values that do not have the `Display` trait implemented but do have the `Debug` trait. Alternatively you can use the `dbg!` macro to print these types of variables.

```rust, ignore
println!("WALLET 1 PROVIDER {:?}", wallet_1.get_provider().unwrap());
dbg!("WALLET 1 PROVIDER {}", wallet_1.get_provider().unwrap());
```

To print more complex types that don't have it already, you can implement your own formatted display method with the `fmt` library from the Rust standard library.

```rust, ignore
use std::fmt;

struct Point {
    x: u64,
    y: u64,
}

// add print functionality with the fmt library 
impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "value of x: {}, value of y: {}", self.x, self.y)
    }
}

let p = Point {x: 1, y: 2};
println!("POINT: {}", p);
```

## Run Commands

You can run your tests to see if they pass or fail with

```
cargo test
```

If you want to see anything printed to the console from your tests, use the `nocapture` flag.

```
cargo test -- --nocapture
```
