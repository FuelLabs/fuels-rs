#[cfg(feature = "fuel-core-lib")]
use fuel_core::service::{Config, FuelService};

#[cfg(not(feature = "fuel-core-lib"))]
use crate::node::{Config, FuelService};

use fuels_signers::provider::Provider;

use fuel_gql_client::fuel_tx::{Receipt, Transaction};
use fuels_contract::script::Script;
use fuels_core::parameters::TxParameters;
use fuels_types::errors::Error;

/// Run the script binary located at `binary_filepath` and return its resulting receipts,
/// without having to setup a node or contract bindings.
pub async fn run_compiled_script(
    binary_filepath: &str,
    tx_params: TxParameters,
    provider: Option<Provider>,
) -> Result<Vec<Receipt>, Error> {
    let script_binary = std::fs::read(binary_filepath)?;
    let server = FuelService::new_node(Config::local_node()).await.unwrap();
    let provider = provider.unwrap_or(Provider::connect(server.bound_address.to_string()).await?);

    let script = build_script(script_binary, tx_params);

    script.call(&provider).await
}

fn build_script(script_binary: Vec<u8>, tx_params: TxParameters) -> Script {
    let tx = Transaction::Script {
        gas_price: tx_params.gas_price,
        gas_limit: tx_params.gas_limit,
        maturity: tx_params.maturity,
        receipts_root: Default::default(),
        script: script_binary, // Pass the compiled script into the tx
        script_data: vec![],
        inputs: vec![],
        outputs: vec![],
        witnesses: vec![vec![].into()],
        metadata: None,
    };

    Script::new(tx)
}

#[cfg(test)]
mod tests {
    use crate::script::run_compiled_script;
    use crate::{
        setup_single_asset_coins, setup_test_provider, DEFAULT_COIN_AMOUNT, DEFAULT_NUM_COINS,
    };
    use fuels_core::constants::BASE_ASSET_ID;
    use fuels_core::parameters::TxParameters;
    use fuels_signers::WalletUnlocked;
    use fuels_types::errors::Error;

    #[tokio::test]
    async fn test_run_compiled_script() -> Result<(), Error> {
        // ANCHOR: run_compiled_script
        let path_to_bin = "../fuels/tests/test_projects/logging/out/debug/logging.bin";
        let return_val = run_compiled_script(path_to_bin, TxParameters::default(), None).await?;

        let correct_hex =
            hex::decode("ef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a")?;

        assert_eq!(correct_hex, return_val[0].data().unwrap());
        // ANCHOR_END: run_compiled_script
        Ok(())
    }

    #[tokio::test]
    async fn test_run_compiled_script_with_custom_provider() -> Result<(), Error> {
        let path_to_bin = "../fuels/tests/test_projects/logging/out/debug/logging.bin";

        let wallet = WalletUnlocked::new_random(None);

        let coins = setup_single_asset_coins(
            wallet.address(),
            BASE_ASSET_ID,
            DEFAULT_NUM_COINS,
            DEFAULT_COIN_AMOUNT,
        );
        let (provider, _) = setup_test_provider(coins, vec![], None).await;

        let return_val =
            run_compiled_script(path_to_bin, TxParameters::default(), Some(provider)).await?;

        let correct_hex =
            hex::decode("ef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a")?;

        assert_eq!(correct_hex, return_val[0].data().unwrap());
        Ok(())
    }
}
