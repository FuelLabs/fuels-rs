library;

pub struct SharedStruct1<T> {
    a: T,
}

pub struct SharedStruct2<K> {
    a: u32,
    b: SharedStruct1<K>,
}

pub enum SharedEnum<L> {
    a: u64,
    b: SharedStruct2<L>,
}
