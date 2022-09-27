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
    fn vec_in_struct(arg: SomeStruct<Vec<u32>>) -> SomeStruct<Vec<u32>>;
    fn array_in_vec(arg: Vec<[u64; 2]>) -> Vec<[u64; 2]>;
    fn vec_in_array(arg: [Vec<u32>; 2]) -> [Vec<u32>; 2];
    fn enum_in_vec(arg: Vec<SomeEnum<u32>>) -> Vec<SomeEnum<u32>>;
    fn vec_in_enum(arg: SomeEnum<Vec<u32>>) -> SomeEnum<Vec<u32>>;
    fn tuple_in_vec(arg: Vec<(u32, u32)>) -> Vec<(u32, u32)>;
    fn vec_in_tuple(arg: (Vec<u32>, Vec<u32>)) -> (Vec<u32>, Vec<u32>);

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
            Option::None => {
                match other {
                    Option::None => true,
                    _ => false,
                }
            }
        }
    }
}

impl Eq for (u32, u32) {
    fn eq(self, other: Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl Eq for Option<(u32, u32)> {
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
            Option::None => {
                match other {
                    Option::None => true,
                    _ => false,
                }
            }
        }
    }
}

impl Eq for SomeEnum<u32> {
    fn eq(self, other: Self) -> bool {
        match self {
            SomeEnum::a(val) => {
                match other {
                    SomeEnum::a(other_val) => {
                        val == other_val
                    },
                    _ => false,
                }
            },
            _ => {
                assert(false);
                false
            }
        }
    }
}

impl Eq for Option<SomeEnum<u32>> {
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
            Option::None => {
                match other {
                    Option::None => true,
                    _ => false,
                }
            }
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

impl Eq for (Vec<u32>, Vec<u32>) {
    fn eq(self, other: Self) -> bool {
        self.0 == other.0 && self.1 == other.1
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
            Option::None => {
                match other {
                    Option::None => true,
                    _ => false,
                }
            }
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
            Option::None => {
                match other {
                    Option::None => true,
                    _ => false,
                }
            }
        }
    }
}

impl Eq for [Vec<u32>; 2] {
    fn eq(self, other: Self) -> bool {
        let mut i = 0;
        while i < 2 {
            if self[i] != other[i] {
                return false;
            }
            i += 1;
        }
        true
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

impl Eq for [u64; 2] {
    fn eq(self, other: Self) -> bool {
        let mut i = 0;
        while i < 2 {
            if self[i] != other[i] {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl Eq for Option<[u64; 2]> {
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
            Option::None => {
                match other {
                    Option::None => true,
                    _ => false,
                }
            }
        }
    }
}

impl Eq for Vec<[u64; 2]> {
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

impl Eq for SomeEnum<Vec<u32>> {
    fn eq(self, other: Self) -> bool {
        match self {
            SomeEnum::a(val) => {
                match other {
                    SomeEnum::a(other_val) => {
                        val == other_val
                    },
                    _ => false,
                }
            },
            _ => {
                assert(false);
                false
            }
        }
    }
}

impl Eq for Vec<SomeEnum<u32>> {
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

impl Eq for Vec<(u32, u32)> {
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
        log_vec(expected);

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
    fn vec_in_struct(arg: SomeStruct<Vec<u32>>) -> SomeStruct<Vec<u32>> {
        let expected = SomeStruct {
            a: expected_vec(),
        };

        assert(arg.a == expected.a);
        log_vec(expected.a);
        expected
    }
    fn array_in_vec(arg: Vec<[u64; 2]>) -> Vec<[u64; 2]> {
        let mut expected = ~Vec::new();
        expected.push([0, 1]);
        expected.push([0, 1]);

        assert(arg == expected);
        log_vec(expected);
        expected
    }

    fn vec_in_array(arg: [Vec<u32>; 2]) -> [Vec<u32>; 2] {
        let expected = [
            expected_vec(),
            expected_vec(),
        ];

        assert(expected == arg);

        log_vec(expected[0]);
        log_vec(expected[1]);
        expected
    }
    fn vec_in_enum(arg: SomeEnum<Vec<u32>>) -> SomeEnum<Vec<u32>> {
        let vec = expected_vec();
        let expected = SomeEnum::a(vec);

        assert(expected == arg);
        log_vec(vec);
        expected
    }
    fn enum_in_vec(arg: Vec<SomeEnum<u32>>) -> Vec<SomeEnum<u32>> {
        let mut expected = ~Vec::new();
        expected.push(SomeEnum::a(0));
        expected.push(SomeEnum::a(1));

        assert(arg == expected);

        log_vec(expected);
        expected
    }

    fn tuple_in_vec(arg: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
        let mut expected = ~Vec::new();
        expected.push((0, 0));
        expected.push((1, 1));

        assert(arg == expected);
        log_vec(expected);
        expected
    }

    fn vec_in_tuple(arg: (Vec<u32>, Vec<u32>)) -> (Vec<u32>, Vec<u32>) {
        let expected = (
            expected_vec(),
            expected_vec(),
        );

        assert(arg == expected);

        log_vec(expected.0);
        log_vec(expected.1);
        expected
    }
}

enum SomeEnum<T> {
    a: T,
}
