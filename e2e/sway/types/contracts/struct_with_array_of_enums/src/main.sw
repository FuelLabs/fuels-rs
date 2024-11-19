contract;

enum EnumWithNative {
    Checked: (),
    Pending: (),
}

impl Eq for EnumWithNative {
    fn eq(self, other: Self) -> bool {
        match (self, other) {
            (EnumWithNative::Checked, EnumWithNative::Checked) => true,
            (EnumWithNative::Pending, EnumWithNative::Pending) => true,
            _ => false,
        }
    }
}

struct StructWithEnumArray {
    a: [EnumWithNative; 3],
}

impl Eq for [EnumWithNative; 3] {
    fn eq(self, other: Self) -> bool {
        self[0] == other[0] && self[1] == other[1] && self[2] == other[2]
    }
}

impl Eq for StructWithEnumArray {
    fn eq(self, other: Self) -> bool {
        self.a == other.a
    }
}

abi TestContract {
    fn return_struct_with_enum_array(x: StructWithEnumArray) -> StructWithEnumArray;
}

impl TestContract for Contract {
    fn return_struct_with_enum_array(x: StructWithEnumArray) -> StructWithEnumArray {
        const INPUT_ENUM = EnumWithNative::Checked;
        const INPUT: StructWithEnumArray = StructWithEnumArray {
            a: [INPUT_ENUM, INPUT_ENUM, INPUT_ENUM],
        };
        assert(x == INPUT);

        const EXPECTED_ENUM = EnumWithNative::Pending;
        const EXPECTED: StructWithEnumArray = StructWithEnumArray {
            a: [EXPECTED_ENUM, EXPECTED_ENUM, EXPECTED_ENUM],
        };

        EXPECTED
    }
}
