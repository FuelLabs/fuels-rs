use fuels_macros::Parameterize;

#[derive(Parameterize)]
#[FuelsTypesPath]
enum SomeEnum {
    A(u8),
}

#[derive(Parameterize)]
#[FuelsTypesPath = true]
struct SomeStruct {}

fn main() {}
