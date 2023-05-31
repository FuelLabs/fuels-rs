mod abi_decoder;
mod abi_encoder;
mod function_selector;

pub use abi_decoder::*;
pub use abi_encoder::*;
pub use function_selector::*;

use crate::{
    traits::{Parameterize, Tokenizable},
    types::errors::Result,
};

pub fn try_from_bytes<T>(bytes: &[u8]) -> Result<T>
where
    T: Parameterize + Tokenizable,
{
    let token = ABIDecoder::decode_single(&T::param_type(), bytes)?;

    T::from_token(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        constants::WORD_SIZE,
        types::{Address, AssetId, ContractId},
    };

    #[test]
    fn can_convert_bytes_into_tuple() -> Result<()> {
        let tuple_in_bytes: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];

        let the_tuple: (u64, u32) = try_from_bytes(&tuple_in_bytes)?;

        assert_eq!(the_tuple, (1, 2));

        Ok(())
    }

    #[test]
    fn can_convert_all_from_bool_to_u64() -> Result<()> {
        let bytes: Vec<u8> = vec![0xFF; WORD_SIZE];

        assert!(try_from_bytes::<bool>(&bytes)?);
        assert_eq!(try_from_bytes::<u8>(&bytes)?, u8::MAX);
        assert_eq!(try_from_bytes::<u16>(&bytes)?, u16::MAX);
        assert_eq!(try_from_bytes::<u32>(&bytes)?, u32::MAX);
        assert_eq!(try_from_bytes::<u64>(&bytes)?, u64::MAX);

        Ok(())
    }

    #[test]
    fn can_convert_native_types() -> Result<()> {
        let bytes = [0xFF; 32];

        assert_eq!(
            try_from_bytes::<Address>(&bytes)?,
            Address::new(bytes.as_slice().try_into()?)
        );
        assert_eq!(
            try_from_bytes::<ContractId>(&bytes)?,
            ContractId::new(bytes.as_slice().try_into()?)
        );
        assert_eq!(
            try_from_bytes::<AssetId>(&bytes)?,
            AssetId::new(bytes.as_slice().try_into()?)
        );
        Ok(())
    }
}
