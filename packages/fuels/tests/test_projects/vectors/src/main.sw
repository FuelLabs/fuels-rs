contract;
use std::vec::Vec;
use std::mem::addr_of;
use std::logging::log;
use std::option::Option;
use std::assert::assert;

abi MyContract {
    fn u32_vec(arg: Vec<u32>);
    fn struct_in_vec(arg: Vec<SomeStruct<u32>>);
    fn vec_in_struct(arg: SomeStruct<Vec<u32>>);
    fn array_in_vec(arg: Vec<[u64; 2]>);
    fn vec_in_array(arg: [Vec<u32>; 2]);
    fn enum_in_vec(arg: Vec<SomeEnum<u32>>);
    fn vec_in_enum(arg: SomeEnum<Vec<u32>>);
    fn tuple_in_vec(arg: Vec<(u32, u32)>);
    fn vec_in_tuple(arg: (Vec<u32>, Vec<u32>));

    fn vec_in_vec(arg: Vec<Vec<u32>>);
}

struct SomeStruct<T> {
    a: T,
}

fn check_vec(arg: Vec<u32>) {
    assert(arg.len() == 3);

    let mut i = 0;
    while i < arg.len() {
        let option = arg.get(i);
        match option {
            Option::Some(val) => {
                assert(val == i);
            },
            _ => {
                assert(false);
            }
        };
        i += 1;
    }
}

impl MyContract for Contract {
    fn u32_vec(arg: Vec<u32>) {
        check_vec(arg);
    }
    fn vec_in_vec(arg: Vec<Vec<u32>>) {
        assert(arg.len() == 2);

        let mut i = 0;
        while i < arg.len() {
            let option = arg.get(i);
            match option {
                Option::Some(val) => {
                    check_vec(val);
                },
                _ => {
                    assert(false);
                }
            };
            i += 1;
        }
    }
    fn struct_in_vec(arg: Vec<SomeStruct<u32>>) {
        assert(arg.len() == 2);

        let mut i = 0;
        while i < arg.len() {
            let option = arg.get(i);
            match option {
                Option::Some(val) => {
                    assert(val.a == i);
                },
                _ => {
                    assert(false);
                }
            };
            i += 1;
        }
    }
    fn vec_in_struct(arg: SomeStruct<Vec<u32>>) {
        check_vec(arg.a);
    }
    fn array_in_vec(arg: Vec<[u64; 2]>) {
        assert(arg.len() == 2);

        let mut i = 0;
        while i < arg.len() {
            let option = arg.get(i);
            match option {
                Option::Some(val) => {
                    assert(val[0] == 0);
                    assert(val[1] == 1);
                },
                _ => {
                    assert(false);
                }
            };
            i += 1;
        }
    }
    fn vec_in_array(arg: [Vec<u32>; 2]) {
        check_vec(arg[0]);
        check_vec(arg[1]);
    }
    fn vec_in_enum(arg: SomeEnum<Vec<u32>>) {
        match arg {
            SomeEnum::a(val) => {
                check_vec(val);
            },
            _ => {
                assert(false);
            }
        }
    }
    fn enum_in_vec(arg: Vec<SomeEnum<u32>>) {
        assert(arg.len() == 2);

        let mut i = 0;
        while i < arg.len() {
            let option = arg.get(i);
            match option {
                Option::Some(val) => {
                    match val {
                        SomeEnum::a(inner_val) => {
                            assert(inner_val == i);
                        },
                        _ => {
                            assert(false);
                        }
                    }
                },
                _ => {
                    assert(false);
                }
            };
            i += 1;
        }
    }

    fn tuple_in_vec(arg: Vec<(u32, u32)>) {
        assert(arg.len() == 2);

        let mut i = 0;
        while i < arg.len() {
            let option = arg.get(i);
            match option {
                Option::Some(val) => {
                    assert(val.0 == i);
                    assert(val.1 == i);
                },
                _ => {
                    assert(false);
                }
            };
            i += 1;
        }
    }

    fn vec_in_tuple(arg: (Vec<u32>, Vec<u32>)) {
        check_vec(arg.0);
        check_vec(arg.1);
    }
}

enum SomeEnum<T> {
    a: T,
}
