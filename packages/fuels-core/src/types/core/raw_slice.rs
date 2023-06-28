#[derive(Debug, PartialEq, Clone, Eq)]
// `RawSlice` is a mapping of the contract type "untyped raw slice" -- currently the only way of
// returning dynamically sized data from a script.
pub struct RawSlice(pub Vec<u64>);

impl From<RawSlice> for Vec<u64> {
    fn from(raw_slice: RawSlice) -> Vec<u64> {
        raw_slice.0
    }
}

impl PartialEq<Vec<u64>> for RawSlice {
    fn eq(&self, other: &Vec<u64>) -> bool {
        self.0 == *other
    }
}

impl PartialEq<RawSlice> for Vec<u64> {
    fn eq(&self, other: &RawSlice) -> bool {
        *self == other.0
    }
}
