#[cfg(test)]
mod tests {
    use fuels::prelude::Result;

    #[tokio::test]
    async fn script_configurables() -> Result<()> {
        use fuels::{
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

        let configurables = MyScriptConfigurables::default()
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

    #[tokio::test]
    async fn script_configurables_reader() -> Result<()> {
        use fuels::prelude::*;

        // ANCHOR: script_configurables_reader
        abigen!(Script(
            name = "MyScript",
            abi = "e2e/sway/scripts/script_configurables/out/release/script_configurables-abi.json"
        ));

        let configurables_reader = MyScriptConfigurablesReader::load_from(
            "../../e2e/sway/scripts/script_configurables/out/release/script_configurables.bin",
        )?;

        let some_bool = configurables_reader.BOOL()?;
        let some_u8 = configurables_reader.U8()?;
        let some_str_4 = configurables_reader.STR_4()?;
        let some_array = configurables_reader.ARRAY()?;
        // ANCHOR_END: script_configurables_reader

        let str_4: fuels::types::SizedAsciiString<4> = "fuel".try_into()?;
        assert!(some_bool);
        assert_eq!(some_u8, 8);
        assert_eq!(some_str_4, str_4);
        assert_eq!(some_array, [253, 254, 255]);

        Ok(())
    }
}
