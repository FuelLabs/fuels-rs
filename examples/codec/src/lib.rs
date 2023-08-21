#[cfg(test)]
mod tests {
    use fuels::{
        core::{codec::ABIEncoder, traits::Tokenizable},
        macros::*,
        prelude::*,
        types::unresolved_bytes::UnresolvedBytes,
    };

    #[test]
    fn encoding_a_type() -> Result<()> {
        #[derive(Tokenizable)]
        struct MyStruct {
            field: u64,
        }

        let instance = MyStruct { field: 101 };
        let encoded: UnresolvedBytes = ABIEncoder::encode(&[instance.into_token()])?;
        let load_memory_address: u64 = 0x100;
        let _: Vec<u8> = encoded.resolve(load_memory_address);

        Ok(())
    }
}
