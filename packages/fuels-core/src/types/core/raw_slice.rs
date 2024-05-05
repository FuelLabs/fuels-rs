#[derive(Debug, PartialEq, Clone, Eq)]
pub struct RawSlice(pub Vec<u8>);

impl From<RawSlice> for Vec<u8> {
    fn from(raw_slice: RawSlice) -> Vec<u8> {
        raw_slice.0
    }
}

impl PartialEq<Vec<u8>> for RawSlice {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.0 == *other
    }
}

impl PartialEq<RawSlice> for Vec<u8> {
    fn eq(&self, other: &RawSlice) -> bool {
        *self == other.0
    }
}
