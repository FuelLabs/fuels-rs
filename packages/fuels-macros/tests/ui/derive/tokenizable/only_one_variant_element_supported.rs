use fuels_macros::Tokenizable;

#[derive(Tokenizable)]
enum SomeEnum {
    // problem because no elements present
    B(),
}

#[derive(Tokenizable)]
enum AnotherEnum {
    A(u64, u32),
}

fn main() {}
