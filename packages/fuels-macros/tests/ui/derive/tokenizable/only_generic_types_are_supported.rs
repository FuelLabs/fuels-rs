use fuels_macros::Tokenizable;

#[derive(Tokenizable)]
enum SomeEnum<const T: usize> {
    A,
}

#[derive(Tokenizable)]
enum AnotherEnum<'a> {
    A(&'a u64),
}

#[derive(Tokenizable)]
struct SomeStruct<const T: usize> {}

#[derive(Tokenizable)]
struct AnotherStruct<'a> {
    a: &'a u64,
}

fn main() {}
