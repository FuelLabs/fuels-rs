/// This type is used to specify the fn_selector and name
/// of methods on contracts at compile time, exported by the abigen! macro
#[derive(Debug, Clone, Copy)]
pub struct MethodDescriptor {
    /// The name of the method.
    pub name: &'static str,
    /// The function selector of the method.
    pub fn_selector: &'static [u8],
}

impl MethodDescriptor {
    /// Returns the function selector of the method.
    pub const fn fn_selector(&self) -> &'static [u8] {
        self.fn_selector
    }

    /// Returns the name of the method.
    pub const fn name(&self) -> &'static str {
        self.name
    }
}
