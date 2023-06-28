use fuels_macros::Tokenizable;

#[derive(Tokenizable)]
enum SomeEnum {
    A,
    B { something: u64 },
}

fn main() {}
