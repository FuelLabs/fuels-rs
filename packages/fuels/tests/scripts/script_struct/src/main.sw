script;

struct MyStruct {
    number: u64,
    boolean: bool,
}

fn main(my_struct: MyStruct) -> u64 {
    if my_struct.boolean {
        my_struct.number
    } else {
        0
    }
}
