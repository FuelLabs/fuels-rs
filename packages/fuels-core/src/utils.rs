pub mod constants;
pub mod offsets;

use crate::{
    abi_decoder::ABIDecoder,
    traits::{Parameterize, Tokenizable},
    types::{errors::Result, ByteArray},
};

use sha2::{Digest, Sha256};

/// Hashes an encoded function selector using SHA256 and returns the first 4 bytes.
/// The function selector has to have been already encoded following the ABI specs defined
/// [here](https://github.com/FuelLabs/fuel-specs/blob/1be31f70c757d8390f74b9e1b3beb096620553eb/specs/protocol/abi.md)
pub fn first_four_bytes_of_sha256_hash(string: &str) -> ByteArray {
    let string_as_bytes = string.as_bytes();
    let mut hasher = Sha256::new();
    hasher.update(string_as_bytes);
    let result = hasher.finalize();
    let mut output = ByteArray::default();
    output[4..].copy_from_slice(&result[..4]);
    output
}

pub fn try_from_bytes<T>(bytes: &[u8]) -> Result<T>
where
    T: Parameterize + Tokenizable,
{
    let token = ABIDecoder::decode_single(&T::param_type(), bytes)?;

    T::from_token(token)
}

#[cfg(test)]
mod tests {
    use crate::{
        constants::WORD_SIZE,
        types::{errors::Result, Address, AssetId, ContractId},
    };

    use crate::try_from_bytes;

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
