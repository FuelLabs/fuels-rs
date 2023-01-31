#[cfg(test)]
mod tests {
    use std::str::FromStr;

    #[cfg(feature = "fuel-core-lib")]
    use fuels::prelude::Config;
    use fuels::prelude::Result;

    #[tokio::test]
    async fn bytes32() -> Result<()> {
        // ANCHOR: bytes32
        use std::str::FromStr;

        use fuels::tx::Bytes32;

        // Zeroed Bytes32
        let b256 = Bytes32::zeroed();

        // Grab the inner `[u8; 32]` from
        // `Bytes32` by dereferencing (i.e. `*`) it.
        assert_eq!([0u8; 32], *b256);

        // From a `[u8; 32]`.
        let my_slice = [1u8; 32];
        let b256 = Bytes32::new(my_slice);
        assert_eq!([1u8; 32], *b256);

        // From a hex string.
        let hex_str = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let b256 = Bytes32::from_str(hex_str).expect("failed to create Bytes32 from string");
        assert_eq!([0u8; 32], *b256);
        // ANCHOR_END: bytes32

        // ANCHOR: bytes32_format
        let b256_string = b256.to_string();
        let b256_hex_string = format!("{:#x}", b256);
        // ANCHOR_END: bytes32_format

        assert_eq!(hex_str[2..], b256_string);
        assert_eq!(hex_str, b256_hex_string);

        Ok(())
    }
    #[tokio::test]
    async fn address() -> Result<()> {
        // ANCHOR: address
        use std::str::FromStr;

        use fuels::tx::Address;

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
        let hex_str = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let address = Address::from_str(hex_str).expect("failed to create Address from string");
        assert_eq!([0u8; 32], *address);
        // ANCHOR_END: address
        Ok(())
    }
    #[tokio::test]
    async fn bech32() -> Result<()> {
        // ANCHOR: bech32
        use fuels::{
            prelude::Bech32Address,
            tx::{Address, Bytes32},
        };

        // New from HRP string and a hash
        let hrp = "fuel";
        let my_slice = [1u8; 32];
        let _bech32_address = Bech32Address::new(hrp, my_slice);

        // Note that you can also pass a hash stored as Bytes32 to new:
        let my_hash = Bytes32::new([1u8; 32]);
        let _bech32_address = Bech32Address::new(hrp, my_hash);

        // From a string.
        let address = "fuel1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqsx2mt2";
        let bech32_address =
            Bech32Address::from_str(address).expect("failed to create Bech32 address from string");
        assert_eq!([0u8; 32], *bech32_address.hash());

        // From Address
        let plain_address = Address::new([0u8; 32]);
        let bech32_address = Bech32Address::from(plain_address);
        assert_eq!([0u8; 32], *bech32_address.hash());

        // Convert to Address
        let _plain_address: Address = bech32_address.into();

        // ANCHOR_END: bech32

        Ok(())
    }
    #[tokio::test]
    async fn asset_id() -> Result<()> {
        // ANCHOR: asset_id
        use std::str::FromStr;

        use fuels::tx::AssetId;

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
        let hex_str = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let asset_id = AssetId::from_str(hex_str).expect("failed to create AssetId from string");
        assert_eq!([0u8; 32], *asset_id);
        // ANCHOR_END: asset_id
        Ok(())
    }
    #[tokio::test]
    async fn contract_id() -> Result<()> {
        // ANCHOR: contract_id
        use std::str::FromStr;

        use fuels::tx::ContractId;

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
        let hex_str = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let contract_id =
            ContractId::from_str(hex_str).expect("failed to create ContractId from string");
        assert_eq!([0u8; 32], *contract_id);
        // ANCHOR_END: contract_id
        Ok(())
    }

    #[tokio::test]
    async fn type_conversion() -> Result<()> {
        // ANCHOR: type_conversion
        use fuels::tx::{AssetId, ContractId};

        let contract_id = ContractId::new([1u8; 32]);

        let asset_id: AssetId = AssetId::new(*contract_id);

        assert_eq!([1u8; 32], *asset_id);
        // ANCHOR_END: type_conversion
        Ok(())
    }
}
