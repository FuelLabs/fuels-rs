pub mod call_response;
pub mod call_utils;
pub mod contract;
pub mod logs;
pub mod script_calls;

#[derive(Debug, Default)]
pub struct Configurables {
    pub offsets_with_data: Vec<(u64, Vec<u8>)>,
}

pub fn replace_configurables(configurables: Configurables, binary: &mut Vec<u8>) {
    configurables
        .offsets_with_data
        .iter()
        .for_each(|(offset, data)| {
            let offset = *offset as usize;

            binary.splice(offset..offset + data.len(), data.iter().cloned());
        });
}
