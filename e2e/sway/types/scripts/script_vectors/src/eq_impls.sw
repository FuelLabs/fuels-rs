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
