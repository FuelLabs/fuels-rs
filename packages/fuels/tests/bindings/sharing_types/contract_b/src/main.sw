contract;
use shared_lib::*;

struct UniqueStructToContractB<T> {
    a: T,
}

struct StructSameNameButDifferentInternals {
    a: [u64; 1],
}

#[allow(dead_code)]
enum EnumSameNameButDifferentInternals {
    a: [u64; 1],
}

abi MyContract {
    fn uses_shared_type(
        arg1: SharedStruct2<u32>,
        arg2: SharedEnum<u64>,
    ) -> (SharedStruct2<u32>, SharedEnum<u64>);
    fn uses_types_that_share_only_names(
        arg1: StructSameNameButDifferentInternals,
        arg2: EnumSameNameButDifferentInternals,
    ) -> (StructSameNameButDifferentInternals, EnumSameNameButDifferentInternals);
    fn uses_shared_type_inside_owned_one(
        arg1: UniqueStructToContractB<SharedStruct2<u8>>,
    ) -> UniqueStructToContractB<SharedStruct2<u8>>;
}

impl MyContract for Contract {
    fn uses_shared_type(
        arg1: SharedStruct2<u32>,
        arg2: SharedEnum<u64>,
    ) -> (SharedStruct2<u32>, SharedEnum<u64>) {
        (arg1, arg2)
    }
    fn uses_types_that_share_only_names(
        arg1: StructSameNameButDifferentInternals,
        arg2: EnumSameNameButDifferentInternals,
    ) -> (StructSameNameButDifferentInternals, EnumSameNameButDifferentInternals) {
        (arg1, arg2)
    }
    fn uses_shared_type_inside_owned_one(
        arg1: UniqueStructToContractB<SharedStruct2<u8>>,
    ) -> UniqueStructToContractB<SharedStruct2<u8>> {
        arg1
    }
}
