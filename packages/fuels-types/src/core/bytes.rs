#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Byte(pub u8);
impl From<Byte> for u8 {
    fn from(byte: Byte) -> u8 {
        byte.0
    }
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct RawUntypedPtr(pub u64);

impl From<RawUntypedPtr> for u64 {
    fn from(ptr: RawUntypedPtr) -> Self {
        u64::from(ptr.0)
    }
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Bytes(pub Vec<u8>);

impl From<Bytes> for Vec<u8> {
    fn from(raw_slice: Bytes) -> Vec<u8> {
        raw_slice.0
    }
}

impl PartialEq<Vec<u8>> for Bytes {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.0 == *other
    }
}

impl PartialEq<Bytes> for Vec<u8> {
    fn eq(&self, other: &Bytes) -> bool {
        *self == other.0
    }
}
