library data_structures;

pub struct SomeStruct<T> {
    a: T,
}

pub enum SomeEnum<T> {
    a: T,
}

// ANCHOR: sway_nested_vec_types
pub struct Child {
    grandchild: Vec<u32>,
    info: Vec<u32>,
}
pub struct Parent {
    child: Child,
    info: Vec<u32>,
}
// ANCHOR_END: sway_nested_vec_types
