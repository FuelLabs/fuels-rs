#[cfg(test)]
mod tests {
    use fuels::{core::codec::DecoderConfig, types::errors::Result};

    #[test]
    fn encoding_a_type() -> Result<()> {
        //ANCHOR: encoding_example
        use fuels::{
            core::{codec::ABIEncoder, traits::Tokenizable},
            macros::Tokenizable,
            types::unresolved_bytes::UnresolvedBytes,
        };

        #[derive(Tokenizable)]
        struct MyStruct {
            field: u64,
        }

        let instance = MyStruct { field: 101 };
        let encoded: UnresolvedBytes = ABIEncoder::encode(&[instance.into_token()])?;
        let load_memory_address: u64 = 0x100;
        let _: Vec<u8> = encoded.resolve(load_memory_address);
        //ANCHOR_END: encoding_example

        Ok(())
    }
    #[test]
    fn encoding_via_macro() -> Result<()> {
        //ANCHOR: encoding_example_w_macro
        use fuels::{core::codec::calldata, macros::Tokenizable};

        #[derive(Tokenizable)]
        struct MyStruct {
            field: u64,
        }
        let _: Vec<u8> = calldata!(MyStruct { field: 101 }, MyStruct { field: 102 })?;
        //ANCHOR_END: encoding_example_w_macro

        Ok(())
    }

    #[test]
    fn decoding_example() -> Result<()> {
        // ANCHOR: decoding_example
        use fuels::{
            core::{
                codec::ABIDecoder,
                traits::{Parameterize, Tokenizable},
            },
            macros::{Parameterize, Tokenizable},
            types::Token,
        };

        #[derive(Parameterize, Tokenizable)]
        struct MyStruct {
            field: u64,
        }

        let bytes: &[u8] = &[0, 0, 0, 0, 0, 0, 0, 101];

        let token: Token = ABIDecoder::default().decode(&MyStruct::param_type(), bytes)?;

        let _: MyStruct = MyStruct::from_token(token)?;
        // ANCHOR_END: decoding_example

        Ok(())
    }

    #[test]
    fn decoding_example_try_into() -> Result<()> {
        // ANCHOR: decoding_example_try_into
        use fuels::macros::{Parameterize, Tokenizable, TryFrom};

        #[derive(Parameterize, Tokenizable, TryFrom)]
        struct MyStruct {
            field: u64,
        }

        let bytes: &[u8] = &[0, 0, 0, 0, 0, 0, 0, 101];

        let _: MyStruct = bytes.try_into()?;
        // ANCHOR_END: decoding_example_try_into

        Ok(())
    }

    #[test]
    fn configuring_the_decoder() -> Result<()> {
        // ANCHOR: configuring_the_decoder

        use fuels::core::codec::ABIDecoder;

        ABIDecoder::new(DecoderConfig {
            max_depth: 5,
            max_tokens: 100,
        });
        // ANCHOR_END: configuring_the_decoder

        Ok(())
    }
}
