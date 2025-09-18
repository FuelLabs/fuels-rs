/// This type is used to specify the fn_selector and name
/// of methods on contracts at compile time, exported by the abigen! macro
#[derive(Debug, Clone, Copy)]
pub struct MethodDescriptor {
    pub name: &'static str,
    pub fn_selector: &'static [u8],
}

impl MethodDescriptor {
    pub const fn fn_selector(&self) -> &'static [u8] {
        self.fn_selector
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }
}
