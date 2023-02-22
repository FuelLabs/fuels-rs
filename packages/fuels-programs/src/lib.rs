pub mod call_response;
pub mod call_utils;
pub mod contract;
pub mod logs;
pub mod script_calls;

#[derive(Debug, Default)]
pub struct Configurables {
    offsets_with_data: Vec<(u64, Vec<u8>)>,
}

impl Configurables {
    pub fn new(offsets_with_data: Vec<(u64, Vec<u8>)>) -> Self {
        Self { offsets_with_data }
    }

    pub fn update_constants_in(&self, binary: &mut [u8]) {
        for (offset, data) in &self.offsets_with_data {
            let offset = *offset as usize;
            binary[offset..offset + data.len()].copy_from_slice(data)
        }
    }
}
