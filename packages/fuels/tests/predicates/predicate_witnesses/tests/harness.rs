use fuel_tx::Witness;
use fuels::{
    accounts::predicate::Predicate,
    prelude::*,
    types::{
        transaction_builders::{ScriptTransactionBuilder, TransactionBuilder},
        Bits256, EvmAddress,
    },
};

use ethers_core::{
    rand::thread_rng,
    types::{Signature, U256},
};
use ethers_signers::{LocalWallet, Signer as EthSigner};

const PREDICATE_BINARY_PATH: &str = "./out/debug/signature-predicate.bin";

abigen!(Predicate(
    name = "MyPredicate",
    abi = "out/debug/signature-predicate-abi.json"
));

fn convert_eth_address(eth_wallet_address: &[u8]) -> [u8; 32] {
    let mut address: [u8; 32] = [0; 32];
    address[12..].copy_from_slice(eth_wallet_address);
    address
}

#[tokio::test]
async fn valid_signature_returns_true_for_validating() {
    // Create fuel wallet
    let fuel_wallet = launch_provider_and_get_wallet().await;
    let provider = fuel_wallet.provider().unwrap();

    // Create eth wallet and convert to EVMAddress
    let eth_wallet = LocalWallet::new(&mut thread_rng());
    let padded_eth_address = convert_eth_address(&eth_wallet.address().0);
    let evm_address = EvmAddress::from(Bits256(padded_eth_address));

    // Create the predicate by setting the signer and pass in the witness argument
    let witness_index = 0;
    let amount = 12;
    let asset_id = AssetId::default();
    let configurables = MyPredicateConfigurables::new().with_SIGNER(evm_address);
    let predicate_data = MyPredicateEncoder::encode_data(witness_index);

    // Create a predicate
    let predicate = Predicate::load_from(PREDICATE_BINARY_PATH)
        .unwrap()
        .with_provider(provider.clone())
        .with_data(predicate_data)
        .with_configurables(configurables);

    fuel_wallet
        .transfer(
            &predicate.address().clone(),
            amount,
            asset_id,
            TxParameters::default(),
        )
        .await
        .unwrap();

    // Fetch input predicate
    let inputs_predicate = predicate
        .get_asset_inputs_for_amount(AssetId::default(), amount)
        .await
        .unwrap();

    // Send some amount to the wallet and return the rest to the predicate
    let amount_to_wallet = 6;
    let outputs =
        predicate.get_asset_outputs_for_amount(fuel_wallet.address(), asset_id, amount_to_wallet);

    // Create the Tx
    let network_info = provider.network_info().await.unwrap();
    let tb = ScriptTransactionBuilder::prepare_transfer(
        inputs_predicate,
        outputs,
        TxParameters::default(),
        network_info.clone(),
    );

    let mut tx = tb.build().unwrap();

    // Now that we have the Tx the ethereum wallet must sign the ID
    let tx_id = tx.id(network_info.chain_id());

    let signature = eth_wallet.sign_message(*tx_id).await.unwrap();

    // Convert into compact format for Sway
    let signed_tx: [u8; 64] = compact(&signature);

    // Then we add in the signed data for the witness
    tx.append_witness(Witness::from(signed_tx.to_vec()), &network_info)
        .unwrap();

    // Check predicate balance before
    let balance_before = predicate.get_asset_balance(&asset_id).await.unwrap();
    assert_eq!(balance_before, amount);

    // Execute the Tx
    let tx_id = provider.send_transaction(tx).await.unwrap();
    let _receipts = provider.tx_status(&tx_id).await.unwrap().take_receipts();

    // Check predicate balance after
    let balance_after = predicate.get_asset_balance(&asset_id).await.unwrap();
    assert_eq!(balance_after, amount - amount_to_wallet);
}

// This can probably be cleaned up
fn compact(signature: &Signature) -> [u8; 64] {
    let shifted_parity = U256::from(signature.v - 27) << 255;

    let r = signature.r;
    let y_parity_and_s = shifted_parity | signature.s;

    let mut sig = [0u8; 64];
    let mut r_bytes = [0u8; 32];
    let mut s_bytes = [0u8; 32];
    r.to_big_endian(&mut r_bytes);
    y_parity_and_s.to_big_endian(&mut s_bytes);
    sig[..32].copy_from_slice(&r_bytes);
    sig[32..64].copy_from_slice(&s_bytes);

    sig
}
