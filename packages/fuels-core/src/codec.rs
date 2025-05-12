mod abi_decoder;
mod abi_encoder;
mod abi_formatter;
mod function_selector;
mod logs;
mod utils;

use std::io::Read;

pub use abi_decoder::*;
pub use abi_encoder::*;
pub use abi_formatter::*;
pub use function_selector::*;
pub use logs::*;

use crate::{
    traits::{Parameterize, Tokenizable},
    types::errors::Result,
};

/// Decodes `bytes` into type `T` following the schema defined by T's `Parameterize` impl
pub fn try_from_bytes<T>(bytes: impl Read, decoder_config: DecoderConfig) -> Result<T>
where
    T: Parameterize + Tokenizable,
{
    let token = ABIDecoder::new(decoder_config).decode(&T::param_type(), bytes)?;

    T::from_token(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        constants::WORD_SIZE,
        types::{Address, AsciiString, AssetId, ContractId},
    };

    #[test]
    fn convert_all_from_bool_to_u64() -> Result<()> {
        let bytes = [255; WORD_SIZE];

        macro_rules! test_decode {
            ($($for_type: ident),*) => {
                $(assert_eq!(
                        try_from_bytes::<$for_type>(bytes.as_slice(), DecoderConfig::default())?,
                        $for_type::MAX
                );)*
            };
        }

        assert!(try_from_bytes::<bool>(
            bytes.as_slice(),
            DecoderConfig::default()
        )?);

        test_decode!(u8, u16, u32, u64);

        Ok(())
    }

    #[test]
    fn convert_bytes_into_tuple() -> Result<()> {
        let tuple_in_bytes = [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2];

        let the_tuple: (u64, u32) =
            try_from_bytes(tuple_in_bytes.as_slice(), DecoderConfig::default())?;

        assert_eq!(the_tuple, (1, 2));

        Ok(())
    }

    #[test]
    fn convert_native_types() -> Result<()> {
        let bytes = [255; 32];

        macro_rules! test_decode {
            ($($for_type: ident),*) => {
                $(assert_eq!(
                        try_from_bytes::<$for_type>(bytes.as_slice(), DecoderConfig::default())?,
                        $for_type::new(bytes.as_slice().try_into()?)
                );)*
            };
        }

        test_decode!(Address, ContractId, AssetId);

        Ok(())
    }

    #[test]
    fn string_slice_is_read_in_total() {
        // This was a bug where the decoder read more bytes than it reported, causing the next
        // element to be read incorrectly.

        // given
        #[derive(
            fuels_macros::Tokenizable, fuels_macros::Parameterize, Clone, PartialEq, Debug,
        )]
        #[FuelsCorePath = "crate"]
        #[FuelsTypesPath = "crate::types"]
        struct Test {
            name: AsciiString,
            age: u64,
        }

        let input = Test {
            name: AsciiString::new("Alice".to_owned()).unwrap(),
            age: 42,
        };

        let encoded = ABIEncoder::default()
            .encode(&[input.clone().into_token()])
            .unwrap();

        // when
        let decoded = try_from_bytes::<Test>(encoded.as_slice(), DecoderConfig::default()).unwrap();

        // then
        assert_eq!(decoded, input);
    }
}
