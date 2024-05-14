script;

#[allow(dead_code)]
enum MyEnum {
    One: (),
    Two: (),
    Three: (),
}

fn main(my_enum: MyEnum) -> u64 {
    match my_enum {
        MyEnum::One => 1,
        MyEnum::Two => 2,
        MyEnum::Three => 3,
    }
}
