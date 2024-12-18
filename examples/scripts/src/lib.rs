#[cfg(test)]
mod tests {
    use fuels::prelude::Result;

    #[tokio::test]
    async fn script_configurables() -> Result<()> {
        use fuels::{
            core::codec::EncoderConfig,
            prelude::*,
            types::{Bits256, SizedAsciiString, U256},
        };

        // ANCHOR: script_configurables
        abigen!(Script(
            name = "MyScript",
            abi = "e2e/sway/scripts/script_configurables/out/release/script_configurables-abi.json"
        ));

        let wallet = launch_provider_and_get_wallet().await?;
        let bin_path =
            "../../e2e/sway/scripts/script_configurables/out/release/script_configurables.bin";
        let script_instance = MyScript::new(wallet, bin_path);

        let str_4: SizedAsciiString<4> = "FUEL".try_into()?;
        let new_struct = StructWithGeneric {
            field_1: 16u8,
            field_2: 32,
        };
        let new_enum = EnumWithGeneric::VariantTwo;

        let configurables = MyScriptConfigurables::new(EncoderConfig {
            max_tokens: 5,
            ..Default::default()
        })
        .with_BOOL(false)?
        .with_U8(7)?
        .with_U16(15)?
        .with_U32(31)?
        .with_U64(63)?
        .with_U256(U256::from(8))?
        .with_B256(Bits256([2; 32]))?
        .with_STR_4(str_4.clone())?
        .with_TUPLE((7, false))?
        .with_ARRAY([252, 253, 254])?
        .with_STRUCT(new_struct.clone())?
        .with_ENUM(new_enum.clone())?;

        let response = script_instance
            .with_configurables(configurables)
            .main()
            .call()
            .await?;
        // ANCHOR_END: script_configurables

        let expected_value = (
            false,
            7,
            15,
            31,
            63,
            U256::from(8),
            Bits256([2; 32]),
            str_4,
            (7, false),
            [252, 253, 254],
            new_struct,
            new_enum,
        );

        assert_eq!(response.value, expected_value);

        Ok(())
    }
}
