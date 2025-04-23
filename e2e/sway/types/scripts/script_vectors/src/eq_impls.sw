library;

use ::data_structures::{SomeEnum, SomeStruct};

impl PartialEq for SomeEnum<u32> {
    fn eq(self, other: Self) -> bool {
        match self {
            SomeEnum::a(val) => {
                match other {
                    SomeEnum::a(other_val) => {
                        val == other_val
                    }
                }
            }
        }
    }
}

impl PartialEq for SomeStruct<u32> {
    fn eq(self, other: Self) -> bool {
        self.a == other.a
    }
}

impl PartialEq for [Vec<u32>; 2] {
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

impl PartialEq for [u64; 2] {
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

impl PartialEq for SomeEnum<Vec<u32>> {
    fn eq(self, other: Self) -> bool {
        match self {
            SomeEnum::a(val) => {
                match other {
                    SomeEnum::a(other_val) => {
                        val == other_val
                    }
                }
            }
        }
    }
}

impl PartialEq for SomeStruct<Vec<u32>> {
    fn eq(self, other: Self) -> bool {
        self.a == other.a
    }
}

impl PartialEq for SomeStruct<Vec<Vec<u32>>> {
    fn eq(self, other: Self) -> bool {
        self.a == other.a
    }
}
