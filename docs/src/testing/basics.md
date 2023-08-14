# Testing Basics

If you're new to Rust, you'll want to review these important tools to help you build tests.

## The `assert!` macro

<!-- This section should explain the `assert!` macro -->
<!-- assert:example:start -->
You can use the `assert!` macro to assert certain conditions in your test. This macro invokes `panic!()` and fails the test if the expression inside evaluates to `false`.
<!-- assert:example:end -->

<!-- This section should show an example of the `assert!` macro -->
<!-- assert_code:example:start -->
```rust, ignore
assert!(value == 5);
```
<!-- assert_code:example:end -->

## The `assert_eq!` macro

<!-- This section should show an example of the `assert_eq!` macro -->
<!-- assert_eq:example:start -->
The `assert_eq!` macro works a lot like the `assert` macro, however instead it accepts two values, and panics if those values are not equal.
<!-- assert_eq:example:end -->

<!-- This section should show an example of the `assert_eq!` macro -->
<!-- assert_eq_code:example:start -->
```rust, ignore
assert_eq!(balance, 100);
```
<!-- assert_eq_code:example:end -->

## The `assert_ne!` macro

<!-- This section should show an example of the `assert_ne!` macro -->
<!-- assert_ne:example:start -->
The `assert_ne!` macro works just like the `assert_eq!` macro, but it will panic if the two values are equal.
<!-- assert_ne:example:end -->

<!-- This section should show an example of the `assert_ne!` macro -->
<!-- assert_ne_code:example:start -->
```rust, ignore
assert_ne!(address, 0);
```
<!-- assert_ne_code:example:end -->

## The `println!` macro

<!-- This section should explain how the `println!` macro can be used in tests -->
<!--print_ln:example:start -->
You can use the `println!` macro to print values to the console.
<!--print_ln:example:end -->

<!-- This section should show an example of the `println!` macro -->
<!--print_ln_code:example:start -->
```rust, ignore
println!("WALLET 1 ADDRESS {}", wallet_1.address());
println!("WALLET 1 ADDRESS {:?}", wallet_1.address());
```
<!--print_ln_code:example:end -->

<!-- This section should explain how `{}` and `{:?}` are used in the `println!` macro -->
<!--print_ln_2:example:start -->
Using `{}` will print the value, and using `{:?}` will print the value plus its type.

Using `{:?}` will also allow you to print values that do not have the `Display` trait implemented but do have the `Debug` trait. Alternatively you can use the `dbg!` macro to print these types of variables.
<!--print_ln_2:example:end -->

<!-- This section should show an example of the `println!` and `dbg` macros -->
<!--print_ln_dbg_code:example:start -->
```rust, ignore
println!("WALLET 1 PROVIDER {:?}", wallet_1.provider().unwrap());
dbg!("WALLET 1 PROVIDER {}", wallet_1.provider().unwrap());
```
<!--print_ln_dbg_code:example:end -->

<!-- This section should explain how implement custom fmt -->
<!--fmt:example:start -->
To print more complex types that don't have it already, you can implement your own formatted display method with the `fmt` module from the Rust standard library.
<!--fmt:example:end -->

<!-- This section should show a code example of how implement custom fmt -->
<!--fmt_code:example:start -->
```rust, ignore
use std::fmt;

struct Point {
    x: u64,
    y: u64,
}

// add print functionality with the fmt module 
impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "value of x: {}, value of y: {}", self.x, self.y)
    }
}

let p = Point {x: 1, y: 2};
println!("POINT: {}", p);
```
<!--fmt_code:example:end -->

## Run Commands

You can run your tests to see if they pass or fail with

```shell
cargo test
```

<!-- This section should when outputs are hidden and what the `nocapture` flag does -->
<!--outputs:example:start -->
Outputs will be hidden if the test passes. If you want to see outputs printed from your tests regardless of whether they pass or fail, use the `nocapture` flag.
<!--outputs:example:end -->

```shell
cargo test -- --nocapture
```
