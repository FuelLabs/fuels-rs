use fuels_macros::Parameterize;

#[derive(Parameterize)]
enum SomeEnum<const T: usize> {
    A,
}

#[derive(Parameterize)]
enum AnotherEnum<'a> {
    A(&'a u64),
}

#[derive(Parameterize)]
struct SomeStruct<const T: usize> {}

#[derive(Parameterize)]
struct AnotherStruct<'a> {
    a: &'a u64,
}

fn main() {}
