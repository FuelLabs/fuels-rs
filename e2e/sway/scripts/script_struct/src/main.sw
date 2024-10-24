script;

configurable {
    MY_STRUCT: MyStruct = MyStruct {
        number: 10,
        boolean: true,
    },
    A_NUMBER: u64 = 11,
}

struct MyStruct {
    number: u64,
    boolean: bool,
}

fn main(arg: MyStruct) -> u64 {
    let _calc = MY_STRUCT.number + A_NUMBER;
    if arg.boolean { arg.number } else { 0 }
}
