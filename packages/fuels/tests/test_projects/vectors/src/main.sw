contract;
use std::vec::Vec;
use std::mem::addr_of;
use std::logging::log;
use core::ops::Eq;
use std::option::Option;
use std::assert::assert;

abi MyContract {
    fn u32_vec(arg: Vec<u32>) -> Vec<u32>;
    fn struct_in_vec(arg: Vec<SomeStruct<u32>>) -> Vec<SomeStruct<u32>>;
    fn vec_in_struct(arg: SomeStruct<Vec<u32>>);
    fn array_in_vec(arg: Vec<[u64; 2]>);
    fn vec_in_array(arg: [Vec<u32>; 2]);
    fn enum_in_vec(arg: Vec<SomeEnum<u32>>);
    fn vec_in_enum(arg: SomeEnum<Vec<u32>>);
    fn tuple_in_vec(arg: Vec<(u32, u32)>);
    fn vec_in_tuple(arg: (Vec<u32>, Vec<u32>));

    fn vec_in_vec(arg: Vec<Vec<u32>>) -> Vec<Vec<u32>>;
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

fn log_vec<T>(vec: Vec<T>) {
    let mut i = 0;
    while i < vec.len() {
        let el = vec.get(i);

        match el {
            Option::Some(val) => {
                log(val)
            },
            _ => {
                assert(false);
            }
        };

        i += 1;
    }
}

fn expected_vec() -> Vec<u32> {
    let mut vec = ~Vec::new();
    vec.push(0);
    vec.push(1);
    vec.push(2);
    vec
}

impl Eq for Option<u32> {
    fn eq(self, other: Self) -> bool {
        match self {
            Option::Some(val) => {
                match other {
                    Option::Some(other_val) => {
                        val == other_val
                    },
                    _ => false,
                }
            },
            _ => false,
        }
    }
}

impl Eq for Vec<u32> {
    fn eq(self, other: Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len() {
            if self.get(i) != other.get(i) {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl Eq for Option<Vec<u32>> {
    fn eq(self, other: Self) -> bool {
        match self {
            Option::Some(val) => {
                match other {
                    Option::Some(other_val) => {
                        val == other_val
                    },
                    _ => false,
                }
            },
            _ => false,
        }
    }
}

impl Eq for Vec<Vec<u32>> {
    fn eq(self, other: Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len() {
            if self.get(i) != other.get(i) {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl Eq for SomeStruct<u32> {
    fn eq(self, other: Self) -> bool {
        self.a == other.a
    }
}

impl Eq for Option<SomeStruct<u32>> {
    fn eq(self, other: Self) -> bool {
        match self {
            Option::Some(val) => {
                match other {
                    Option::Some(other_val) => {
                        val == other_val
                    },
                    _ => false,
                }
            },
            _ => false,
        }
    }
}

impl Eq for Vec<SomeStruct<u32>> {
    fn eq(self, other: Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len() {
            if self.get(i) != other.get(i) {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl MyContract for Contract {
    fn u32_vec(arg: Vec<u32>) -> Vec<u32> {
        let expected = expected_vec();

        assert(arg == expected);

        log_vec(expected);
        expected
    }

    fn vec_in_vec(arg: Vec<Vec<u32>>) -> Vec<Vec<u32>> {
        let mut expected = ~Vec::new();
        expected.push(expected_vec());
        expected.push(expected_vec());

        assert(expected == arg);

        log_vec(expected);
        let mut i = 0;
        while i < expected.len() {
            match expected.get(i) {
                Option::Some(val) => {
                    log_vec(val);
                },
                _ => {
                    assert(false);
                }
            }
            i += 1;
        }

        expected
    }

    fn struct_in_vec(arg: Vec<SomeStruct<u32>>) -> Vec<SomeStruct<u32>> {
        let mut expected = ~Vec::new();
        expected.push(SomeStruct { a: 0u32 });
        expected.push(SomeStruct { a: 1u32 });

        assert(arg == expected);

        log_vec(expected);
        expected
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
