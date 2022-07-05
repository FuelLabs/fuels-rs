contract;

enum SomeEnum {
	SomeStr: str[4],
	SomeArr: [u64; 4]
}

abi MyContract {
    fn get_enum_str(my_enum: SomeEnum) -> SomeEnum;
    fn get_enum_arr(my_enum: SomeEnum) -> SomeEnum;
}

impl MyContract for Contract {
    fn get_enum_str(my_enum: SomeEnum) -> SomeEnum {
        my_enum
    }

    fn get_enum_arr(my_enum: SomeEnum) -> SomeEnum {
        my_enum
    }
}