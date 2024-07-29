use std::str::FromStr;

use fuels::{core::traits::Tokenizable, prelude::*, types::Token};

pub fn null_contract_id() -> Bech32ContractId {
    // a bech32 contract address that decodes to [0u8;32]
    Bech32ContractId::from_str("fuel1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqsx2mt2")
        .unwrap()
}

#[tokio::test]
async fn create_struct_from_decoded_tokens() -> Result<()> {
    abigen!(Contract(
        name = "SimpleContract",
        abi = "e2e/sway/types/contracts/nested_structs/out/release/nested_structs-abi.json"
    ));

    let u32_token = Token::U32(10);
    let bool_token = Token::Bool(true);
    let struct_from_tokens = SomeStruct::from_token(Token::Struct(vec![u32_token, bool_token]))?;

    assert_eq!(10, struct_from_tokens.field);
    assert!(struct_from_tokens.field_2);

    Ok(())
}

#[tokio::test]
async fn create_nested_struct_from_decoded_tokens() -> Result<()> {
    abigen!(Contract(
        name = "SimpleContract",
        abi = "e2e/sway/types/contracts/nested_structs/out/release/nested_structs-abi.json"
    ));

    let u32_token = Token::U32(10);
    let bool_token = Token::Bool(true);
    let inner_struct_token = Token::Struct(vec![u32_token, bool_token]);

    let nested_struct_from_tokens = AllStruct::from_token(Token::Struct(vec![inner_struct_token]))?;

    assert_eq!(10, nested_struct_from_tokens.some_struct.field);
    assert!(nested_struct_from_tokens.some_struct.field_2);

    Ok(())
}
