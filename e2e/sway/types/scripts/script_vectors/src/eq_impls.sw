library;

use ::data_structures::{SomeEnum, SomeStruct};
use core::ops::Eq;

impl Eq for (u32, u32) {
    fn eq(self, other: Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl Eq for SomeEnum<u32> {
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

impl Eq for Vec<u32> {
    fn eq(self, other: Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len() {
            if self.get(i).unwrap() != other.get(i).unwrap() {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl Eq for Vec<b256> {
    fn eq(self, other: Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len() {
            if self.get(i).unwrap() != other.get(i).unwrap() {
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

impl Eq for Vec<Vec<u32>> {
    fn eq(self, other: Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len() {
            if self.get(i).unwrap() != other.get(i).unwrap() {
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
            if self.get(i).unwrap() != other.get(i).unwrap() {
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

impl Eq for Vec<[u64; 2]> {
    fn eq(self, other: Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len() {
            if self.get(i).unwrap() != other.get(i).unwrap() {
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
                    }
                }
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
            if self.get(i).unwrap() != other.get(i).unwrap() {
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
            if self.get(i).unwrap() != other.get(i).unwrap() {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl Eq for SomeStruct<Vec<Vec<u32>>> {
    fn eq(self, other: Self) -> bool {
        self.a == other.a
    }
}

impl Eq for Vec<SomeStruct<Vec<Vec<u32>>>> {
    fn eq(self, other: Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        let mut i = 0;
        while i < self.len() {
            if self.get(i).unwrap() != other.get(i).unwrap() {
                return false;
            }
            i += 1;
        }
        true
    }
}
