pub use fuel_abi_types::utils::{ident, safe_ident, TypePath};
pub use source::Source;

mod source;

use fuel_abi_types::abi::full_program::FullProgramABI;

use crate::{error, error::Result, Abi};

pub fn parse_program_abi(abi: &str) -> Result<Abi> {
    let source = Source::parse(abi)?;

    let json_abi_str = source.get()?;
    let abi = FullProgramABI::from_json_abi(&json_abi_str)
        .map_err(|e| error!("malformed `abi`. Did you use `forc` to create it?: ").combine(e))?;
    let path = source.path();

    Ok(Abi { path, abi })
}
