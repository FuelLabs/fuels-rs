#[cfg(test)]
mod tests {
    #[cfg(feature = "fuel-core-lib")]
    use fuels::prelude::Config;
    use fuels::prelude::Error;

    #[tokio::test]
    async fn bytes32() -> Result<(), Error> {
        // ANCHOR: bytes32
        use fuels::tx::Bytes32;
        use std::str::FromStr;

        // Zeroed Bytes32
        let b256 = Bytes32::zeroed();

        // Grab the inner `[u8; 32]` from
        // `Bytes32` by dereferencing (i.e. `*`) it.
        assert_eq!([0u8; 32], *b256);

        // From a `[u8; 32]`.
        let my_slice = [1u8; 32];
        let b256 = Bytes32::new(my_slice);
        assert_eq!([1u8; 32], *b256);

        // From a string.
        let hex_string = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let b256 = Bytes32::from_str(hex_string).expect("failed to create Bytes32 from string");
        assert_eq!([0u8; 32], *b256);
        // ANCHOR_END: bytes32
        Ok(())
    }
    #[tokio::test]
    async fn address() -> Result<(), Error> {
        // ANCHOR: address
        use fuels::tx::Address;
        use std::str::FromStr;

        // Zeroed Bytes32
        let address = Address::zeroed();

        // Grab the inner `[u8; 32]` from
        // `Bytes32` by dereferencing (i.e. `*`) it.
        assert_eq!([0u8; 32], *address);

        // From a `[u8; 32]`.
        let my_slice = [1u8; 32];
        let address = Address::new(my_slice);
        assert_eq!([1u8; 32], *address);

        // From a string.
        let hex_string = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let address = Address::from_str(hex_string).expect("failed to create Address from string");
        assert_eq!([0u8; 32], *address);
        // ANCHOR_END: address
        Ok(())
    }
    #[tokio::test]
    async fn asset_id() -> Result<(), Error> {
        // ANCHOR: asset_id
        use fuels::tx::AssetId;
        use std::str::FromStr;

        // Zeroed Bytes32
        let asset_id = AssetId::zeroed();

        // Grab the inner `[u8; 32]` from
        // `Bytes32` by dereferencing (i.e. `*`) it.
        assert_eq!([0u8; 32], *asset_id);

        // From a `[u8; 32]`.
        let my_slice = [1u8; 32];
        let asset_id = AssetId::new(my_slice);
        assert_eq!([1u8; 32], *asset_id);

        // From a string.
        let hex_string = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let asset_id = AssetId::from_str(hex_string).expect("failed to create AssetId from string");
        assert_eq!([0u8; 32], *asset_id);
        // ANCHOR_END: asset_id
        Ok(())
    }
    #[tokio::test]
    async fn contract_id() -> Result<(), Error> {
        // ANCHOR: contract_id
        use fuels::tx::ContractId;
        use std::str::FromStr;

        // Zeroed Bytes32
        let contract_id = ContractId::zeroed();

        // Grab the inner `[u8; 32]` from
        // `Bytes32` by dereferencing (i.e. `*`) it.
        assert_eq!([0u8; 32], *contract_id);

        // From a `[u8; 32]`.
        let my_slice = [1u8; 32];
        let contract_id = ContractId::new(my_slice);
        assert_eq!([1u8; 32], *contract_id);

        // From a string.
        let hex_string = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let contract_id =
            ContractId::from_str(hex_string).expect("failed to create ContractId from string");
        assert_eq!([0u8; 32], *contract_id);
        // ANCHOR_END: contract_id
        Ok(())
    }

    #[tokio::test]
    async fn type_conversion() -> Result<(), Error> {
        // ANCHOR: type_conversion
        use fuels::tx::{AssetId, ContractId};

        let contract_id = ContractId::new([1u8; 32]);

        let asset_id: AssetId = AssetId::new(*contract_id);

        assert_eq!([1u8; 32], *asset_id);
        // ANCHOR_END: type_conversion
        Ok(())
    }
}
