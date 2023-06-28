use fuels_macros::Parameterize;

#[derive(Parameterize)]
enum SomeEnum {
    // problem because no elements present
    B(),
}

#[derive(Parameterize)]
enum AnotherEnum {
    A(u64, u32),
}

fn main() {}
