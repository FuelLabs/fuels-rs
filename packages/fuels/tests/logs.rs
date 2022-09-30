use fuels::prelude::*;

#[tokio::test]
async fn test_logd_receipts() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_logdata"
    );

    let mut value = [0u8; 32];
    value[0] = 0xFF;
    value[1] = 0xEE;
    value[2] = 0xDD;
    value[12] = 0xAA;
    value[13] = 0xBB;
    value[14] = 0xCC;

    let contract_methods = contract_instance.methods();
    let response = contract_methods
        .use_logd_opcode(Bits256(value), 3, 6)
        .call()
        .await?;
    assert_eq!(response.logs, vec!["ffeedd", "ffeedd000000"]);

    let response = contract_methods
        .use_logd_opcode(Bits256(value), 14, 15)
        .call()
        .await?;
    assert_eq!(
        response.logs,
        vec![
            "ffeedd000000000000000000aabb",
            "ffeedd000000000000000000aabbcc"
        ]
    );

    let response = contract_methods.dont_use_logd().call().await?;
    assert!(response.logs.is_empty());
    Ok(())
}
