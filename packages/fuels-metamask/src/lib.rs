
mod methods;
extern crate web3;

use webbrowser;
use ethers_providers::{Provider, Middleware, RwClient};
use std::convert::TryFrom;
use std::net::{SocketAddr, TcpListener};
use std::str::FromStr;
use axum::{Json, Router};
use ethereum_types::H160;
use ethers::middleware::SignerMiddleware;
use ethers_core::abi::ethereum_types::Signature;
use ethers_core::k256::ecdsa::SigningKey;
use ethers_core::types::{Address, NameOrAddress, TransactionRequest};
use ethers_core::types::transaction::eip2718::TypedTransaction;
use webbrowser::Browser::Chrome;

use ethers_core::types::U256;
use ethers_core::utils::{format_ether, format_units};

use ethers_signers::*;

pub async fn haman_dobro() -> Result<(), Box<dyn std::error::Error>> {

    let client = Provider::try_from("HTTP://127.0.0.1:7545").unwrap();
    // let client = ethers_providers::MAINNET.provider();
    let accounts = client.get_accounts().await?;
    let from = accounts[1];
    let to = accounts[0];

    // let wallet_id = NameOrAddress::Address(Address::from_str("0xdd552DC39Cf22187F8f56560e2A58dd8b409772e").unwrap());
    // let wallet_id = NameOrAddress::Address(Address::from_str("0xb0fa40B1bb8ca88b7A45731CBB5c580E28103d40").unwrap());
    // let from = client.get_balance(wallet_id, None).await?;
    // let from = client.request("", ()).await?;

    let chrome = Address::from_str("0xdd552DC39Cf22187F8f56560e2A58dd8b409772e").unwrap();
    let firefox = Address::from_str("0xE2990793F42826c2D884105dfAD5eC5A094618B1").unwrap();

    dbg!(&from);
    dbg!(&to);

    let s = U256::from("10000000000000000");

    let balance_before = client.get_balance(to, None).await?;
    dbg!("to: ", &balance_before);
    let tx = TransactionRequest::new().to(to).value(s).from(from);
    let receipt = client
        .send_transaction(tx, None)
        .await?                           // PendingTransaction<_>
        .log_msg("Pending transfer hash") // print pending tx hash with message
        .await?;                          // Result<Option<TransactionReceipt>, _>
    let _ = receipt;
    let balance_after = client.get_balance(to, None).await?;
    dbg!("to: ", &balance_after);


    Ok(())
}


// pub async fn ono_sto_je_uspjelo() -> Result<(), Box<dyn std::error::Error>> {
//     let provider = ethers_providers::MAINNET.provider();
//
//     let s = U256::from("100000000000");
//     let chrome = Address::from_str("0xdd552DC39Cf22187F8f56560e2A58dd8b409772e").unwrap();
//     let firefox = Address::from_str("0xE2990793F42826c2D884105dfAD5eC5A094618B1").unwrap();
//
//     // let private_key = "63de71c80eb5def59477c53e7c37c20fb3eb1ddeac7ea5b4883781c8dcf0b2a0";
//     // let wallet: Wallet<SigningKey> =
//     //     "63de71c80eb5def59477c53e7c37c20fb3eb1ddeac7ea5b4883781c8dcf0b2a0".parse().unwrap();
//     // assert_eq!(
//     //     wallet.address(),
//     //     Address::from_str("0xdd552DC39Cf22187F8f56560e2A58dd8b409772e").expect("Decoding failed")
//     // );
//     let mut client = SignerMiddleware::new(provider, wallet);
//     /// // You can sign messages with the key
//     let tx = TransactionRequest::pay(firefox, s);
//     let pending_tx = client.send_transaction(tx, None).await.unwrap();
//
//     dbg!(pending_tx);
// }





#[tokio::test]
async fn test_multiple_args() {


}
