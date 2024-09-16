contract;

use std::hash::*;

#[allow(dead_code)]
struct StructUnusedGeneric<T, K> {
    field: u64,
}

#[allow(dead_code)]
enum EnumUnusedGeneric<T, K> {
    One: u64,
}

struct StructTwoUnusedGenericParams<T, K> {}

#[allow(dead_code)]
enum EnumTwoUnusedGenericParams<T, K> {
    One: (),
}

struct StructUsedAndUnusedGenericParams<T, K, Z> {
    field: K,
}

#[allow(dead_code)]
enum EnumUsedAndUnusedGenericParams<T, K, Z> {
    One: str[3],
    Two: K,
}

struct SimpleGeneric<T> {
    single_generic_param: T,
}

struct PassTheGenericOn<K> {
    one: SimpleGeneric<K>,
}

struct StructWArrayGeneric<L> {
    a: [L; 2],
}

struct StructWTupleGeneric<M> {
    a: (M, M),
}

#[allow(dead_code)]
enum EnumWGeneric<N> {
    A: u64,
    B: N,
}

#[allow(dead_code)]
struct StructWTwoGenerics<T, U> {
    a: T,
    b: U,
}

struct StructWArrWGenericStruct<S> {
    a: [StructWTwoGenerics<S, u8>; 3],
}

#[allow(dead_code)]
struct MegaExample<T, U> {
    a: ([U; 2], T),
    b: Vec<([EnumWGeneric<StructWTupleGeneric<StructWArrayGeneric<PassTheGenericOn<T>>>>; 1], u32)>,
}

impl Hash for str[3] {
    fn hash(self, ref mut state: Hasher) {
        state.write_str(from_str_array(self));
    }
}

abi MyContract {
    fn unused_generic_args(
        arg_1: StructUnusedGeneric<u64, u32>,
        arg_2: EnumUnusedGeneric<u32, u32>,
    );
    fn two_unused_generic_args(
        arg_1: StructTwoUnusedGenericParams<u32, u64>,
        arg_2: EnumTwoUnusedGenericParams<u64, u32>,
    );
    fn used_and_unused_generic_args(
        arg_1: StructUsedAndUnusedGenericParams<u32, u8, u64>,
        arg_2: EnumUsedAndUnusedGenericParams<u64, u8, u32>,
    ) -> (StructUsedAndUnusedGenericParams<u64, u8, u32>, EnumUsedAndUnusedGenericParams<u32, u8, u64>);
    fn struct_w_generic(arg1: SimpleGeneric<u64>) -> SimpleGeneric<u64>;
    fn struct_delegating_generic(arg1: PassTheGenericOn<str[3]>) -> PassTheGenericOn<str[3]>;
    fn struct_w_generic_in_array(arg1: StructWArrayGeneric<u32>) -> StructWArrayGeneric<u32>;
    fn struct_w_generic_in_tuple(arg1: StructWTupleGeneric<u32>) -> StructWTupleGeneric<u32>;

    fn enum_w_generic(arg1: EnumWGeneric<u64>) -> EnumWGeneric<u64>;

    fn complex_test(arg1: MegaExample<str[2], b256>);
    fn array_with_generic_struct(
        arg: StructWArrWGenericStruct<b256>,
    ) -> StructWArrWGenericStruct<b256>;
}

impl MyContract for Contract {
    fn unused_generic_args(
        _arg_1: StructUnusedGeneric<u64, u32>,
        _arg_2: EnumUnusedGeneric<u32, u32>,
    ) {}

    fn two_unused_generic_args(
        _arg_1: StructTwoUnusedGenericParams<u32, u64>,
        _arg_2: EnumTwoUnusedGenericParams<u64, u32>,
    ) {}

    fn used_and_unused_generic_args(
        arg_1: StructUsedAndUnusedGenericParams<u32, u8, u64>,
        arg_2: EnumUsedAndUnusedGenericParams<u64, u8, u32>,
    ) -> (StructUsedAndUnusedGenericParams<u64, u8, u32>, EnumUsedAndUnusedGenericParams<u32, u8, u64>) {
        assert_eq(arg_1.field, 10u8);

        if let EnumUsedAndUnusedGenericParams::Two(val) = arg_2 {
            assert_eq(val, 11u8);
        } else {
            require(
                false,
                "Expected the variant EnumUsedAndUnusedGenericParams::Two",
            );
        }
        (
            StructUsedAndUnusedGenericParams { field: 12u8 },
            EnumUsedAndUnusedGenericParams::Two(13u8),
        )
    }

    fn struct_w_generic(arg1: SimpleGeneric<u64>) -> SimpleGeneric<u64> {
        let expected = SimpleGeneric {
            single_generic_param: 123u64,
        };

        assert(arg1.single_generic_param == expected.single_generic_param);

        expected
    }

    fn struct_delegating_generic(arg1: PassTheGenericOn<str[3]>) -> PassTheGenericOn<str[3]> {
        let expected = PassTheGenericOn {
            one: SimpleGeneric {
                single_generic_param: __to_str_array("abc"),
            },
        };

        assert(
            sha256(from_str_array(expected.one.single_generic_param)) == sha256(from_str_array(arg1.one.single_generic_param)),
        );

        expected
    }

    fn struct_w_generic_in_array(arg1: StructWArrayGeneric<u32>) -> StructWArrayGeneric<u32> {
        let expected = StructWArrayGeneric {
            a: [1u32, 2u32],
        };

        assert(expected.a[0] == arg1.a[0]);
        assert(expected.a[1] == arg1.a[1]);

        expected
    }

    fn struct_w_generic_in_tuple(arg1: StructWTupleGeneric<u32>) -> StructWTupleGeneric<u32> {
        let expected = StructWTupleGeneric {
            a: (1u32, 2u32),
        };
        assert(expected.a.0 == arg1.a.0);
        assert(expected.a.1 == arg1.a.1);

        expected
    }

    fn enum_w_generic(arg1: EnumWGeneric<u64>) -> EnumWGeneric<u64> {
        match arg1 {
            EnumWGeneric::B(value) => {
                assert(value == 10u64);
            }
            _ => {
                assert(false)
            }
        }
        EnumWGeneric::B(10)
    }

    fn complex_test(_arg: MegaExample<str[2], b256>) {}

    fn array_with_generic_struct(
        arg: StructWArrWGenericStruct<b256>,
    ) -> StructWArrWGenericStruct<b256> {
        arg
    }
}
