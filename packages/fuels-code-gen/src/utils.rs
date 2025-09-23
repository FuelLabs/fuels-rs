pub use fuel_abi_types::utils::{TypePath, ident, safe_ident};

pub fn encode_fn_selector(name: &str) -> Vec<u8> {
    let bytes = name.as_bytes().to_vec();
    let len = bytes.len() as u64;

    [len.to_be_bytes().to_vec(), bytes].concat()
}
