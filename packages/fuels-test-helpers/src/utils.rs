use fuel_core_chain_config::{ChainConfig, CoinConfig, MessageConfig, StateConfig};
use fuel_tx::{AssetId, Bytes32, ConsensusParameters, ContractParameters, TxParameters, UtxoId};
use fuel_types::Nonce;
use fuels_accounts::provider::Provider;
use fuels_core::types::{
    bech32::Bech32Address,
    coin::{Coin, CoinStatus},
    message::{Message, MessageStatus},
};
use rand::Fill;

use crate::{AssetConfig, FuelService, NodeConfig};

pub(crate) fn into_coin_configs(coins: Vec<Coin>) -> Vec<CoinConfig> {
    coins
        .into_iter()
        .map(Into::into)
        .collect::<Vec<CoinConfig>>()
}

pub(crate) fn into_message_configs(messages: Vec<Message>) -> Vec<MessageConfig> {
    messages
        .into_iter()
        .map(Into::into)
        .collect::<Vec<MessageConfig>>()
}

/// Create a vector of `num_asset`*`coins_per_asset` UTXOs and a vector of the unique corresponding
/// asset IDs. `AssetId`. Each UTXO (=coin) contains `amount_per_coin` amount of a random asset. The
/// output of this function can be used with `setup_test_provider` to get a client with some
/// pre-existing coins, with `num_asset` different asset ids. Note that one of the assets is the
/// base asset to pay for gas.
pub fn setup_multiple_assets_coins(
    owner: &Bech32Address,
    num_asset: u64,
    coins_per_asset: u64,
    amount_per_coin: u64,
) -> (Vec<Coin>, Vec<AssetId>) {
    let mut rng = rand::thread_rng();
    // Create `num_asset-1` asset ids so there is `num_asset` in total with the base asset
    let asset_ids = (0..(num_asset - 1))
        .map(|_| {
            let mut random_asset_id = AssetId::zeroed();
            random_asset_id
                .try_fill(&mut rng)
                .expect("failed to fill with random data");
            random_asset_id
        })
        .chain([AssetId::zeroed()])
        .collect::<Vec<AssetId>>();

    let coins = asset_ids
        .iter()
        .flat_map(|id| setup_single_asset_coins(owner, *id, coins_per_asset, amount_per_coin))
        .collect::<Vec<Coin>>();

    (coins, asset_ids)
}

/// Create a vector of UTXOs with the provided AssetIds, num_coins, and amount_per_coin
pub fn setup_custom_assets_coins(owner: &Bech32Address, assets: &[AssetConfig]) -> Vec<Coin> {
    let coins = assets
        .iter()
        .flat_map(|asset| {
            setup_single_asset_coins(owner, asset.id, asset.num_coins, asset.coin_amount)
        })
        .collect::<Vec<Coin>>();
    coins
}

/// Create a vector of `num_coins` UTXOs containing `amount_per_coin` amount of asset `asset_id`.
/// The output of this function can be used with `setup_test_provider` to get a client with some
/// pre-existing coins, but with only one asset ID.
pub fn setup_single_asset_coins(
    owner: &Bech32Address,
    asset_id: AssetId,
    num_coins: u64,
    amount_per_coin: u64,
) -> Vec<Coin> {
    let mut rng = rand::thread_rng();

    let coins: Vec<Coin> = (1..=num_coins)
        .map(|_i| {
            let mut r = Bytes32::zeroed();
            r.try_fill(&mut rng)
                .expect("failed to fill with random data");
            let utxo_id = UtxoId::new(r, 0);

            Coin {
                owner: owner.clone(),
                utxo_id,
                amount: amount_per_coin,
                asset_id,
                status: CoinStatus::Unspent,
                block_created: Default::default(),
            }
        })
        .collect();

    coins
}

pub fn setup_single_message(
    sender: &Bech32Address,
    recipient: &Bech32Address,
    amount: u64,
    nonce: Nonce,
    data: Vec<u8>,
) -> Message {
    Message {
        sender: sender.clone(),
        recipient: recipient.clone(),
        nonce,
        amount,
        data,
        da_height: 0,
        status: MessageStatus::Unspent,
    }
}

pub async fn setup_test_provider(
    coins: Vec<Coin>,
    messages: Vec<Message>,
    node_config: Option<NodeConfig>,
    chain_config: Option<ChainConfig>,
) -> fuels_core::types::errors::Result<Provider> {
    let node_config = node_config.unwrap_or_default();
    let chain_config = chain_config.unwrap_or_else(testnet_chain_config);

    let coin_configs = into_coin_configs(coins);
    let message_configs = into_message_configs(messages);

    let state_config = StateConfig {
        coins: coin_configs,
        messages: message_configs,
        ..StateConfig::local_testnet()
    };

    let srv = FuelService::start(node_config, chain_config, state_config).await?;

    let address = srv.bound_address();

    tokio::spawn(async move {
        let _own_the_handle = srv;
        let () = futures::future::pending().await;
    });

    Provider::from(address).await
}

// Testnet ChainConfig with increased tx size and contract size limits
fn testnet_chain_config() -> ChainConfig {
    let mut consensus_parameters = ConsensusParameters::default();
    let tx_params = TxParameters::default().with_max_size(10_000_000);
    let contract_params = ContractParameters::default().with_contract_max_size(1_000_000);
    consensus_parameters.set_tx_params(tx_params);
    consensus_parameters.set_contract_params(contract_params);

    ChainConfig {
        consensus_parameters,
        ..ChainConfig::local_testnet()
    }
}
