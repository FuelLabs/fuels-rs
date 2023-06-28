use fuels_macros::Parameterize;

#[derive(Parameterize)]
enum SomeEnum {
    A,
    B { something: u64 },
}

fn main() {}
