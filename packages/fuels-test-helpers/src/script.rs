#[cfg(test)]
mod tests {
    use crate::{
        setup_single_asset_coins, setup_test_provider, DEFAULT_COIN_AMOUNT, DEFAULT_NUM_COINS,
    };
    use fuels_contract::script::run_script_binary;
    use fuels_core::constants::BASE_ASSET_ID;
    use fuels_signers::WalletUnlocked;
    use fuels_types::errors::Error;

    #[tokio::test]
    async fn test_run_compiled_script() -> Result<(), Error> {
        // ANCHOR: run_compiled_script
        let path_to_bin = "../fuels/tests/logs/logging/out/debug/logging.bin";
        // TODO: use default provider
        let (provider, _) = setup_test_provider(vec![], vec![], None, None).await;
        // Provide `None` to all 3 other arguments so the function uses the default for each
        let return_val =
            run_script_binary(path_to_bin, None, Some(provider), None, None, None).await?;

        let correct_hex =
            hex::decode("ef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a")?;

        assert_eq!(correct_hex, return_val[0].data().unwrap());
        // ANCHOR_END: run_compiled_script
        Ok(())
    }

    #[tokio::test]
    async fn test_run_compiled_script_with_custom_provider() -> Result<(), Error> {
        let path_to_bin = "../fuels/tests/logs/logging/out/debug/logging.bin";

        let wallet = WalletUnlocked::new_random(None);

        let coins = setup_single_asset_coins(
            wallet.address(),
            BASE_ASSET_ID,
            DEFAULT_NUM_COINS,
            DEFAULT_COIN_AMOUNT,
        );
        let (provider, _) = setup_test_provider(coins, vec![], None, None).await;

        let return_val =
            run_script_binary(path_to_bin, None, Some(provider), None, None, None).await?;

        let correct_hex =
            hex::decode("ef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a")?;

        assert_eq!(correct_hex, return_val[0].data().unwrap());
        Ok(())
    }
}
