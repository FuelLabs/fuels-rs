contract;

enum SomeEnum {
	SomeStr: str[4],
	SomeArr: [u64; 7]
}

abi MyContract {
    fn str_inside_enum(my_enum: SomeEnum) -> SomeEnum;
    fn arr_inside_enum(my_enum: SomeEnum) -> SomeEnum;
}

impl MyContract for Contract {
    fn str_inside_enum(my_enum: SomeEnum) -> SomeEnum {
        my_enum
    }

    fn arr_inside_enum(my_enum: SomeEnum) -> SomeEnum {
        my_enum
    }
}