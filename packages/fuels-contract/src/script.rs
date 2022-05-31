use anyhow::Result;
use fuel_gql_client::fuel_tx::{Input, Output, UtxoId};
use fuel_gql_client::fuel_types::{AssetId, Bytes32, ContractId, Word};
use fuel_gql_client::fuel_vm::{
    consts::{REG_CGAS, REG_ONE},
    prelude::Opcode,
    script_with_data_offset,
};
use fuel_gql_client::{
    client::{types::TransactionStatus, FuelClient},
    fuel_tx::{Receipt, Transaction},
};
use fuels_core::constants::{DEFAULT_SPENDABLE_COIN_AMOUNT, WORD_SIZE};
use fuels_core::errors::Error;
use fuels_core::parameters::{CallParameters, TxParameters};
use fuels_core::Selector;

use fuels_signers::{LocalWallet, Signer};
use crate::contract::ContractCall;

/// Script is a very thin layer on top of fuel-client with some
/// extra functionalities needed and provided by the SDK.
pub struct Script {
    pub tx: Transaction,
}

#[derive(Debug, Clone)]
pub struct CompiledScript {
    pub raw: Vec<u8>,
    pub target_network_url: String,
}

impl Script {
    pub fn new(tx: Transaction) -> Self {
        Self { tx }
    }

    pub async fn from_call(
        call: ContractCall,
        contract_id: ContractId,
        tx_parameters: TxParameters,
        wallet: LocalWallet,
    ) -> Self {
        let (script, script_data) = Self::build_script_contents(
            &contract_id,
            &Some(call.encoded_selector),
            &Some(call.encoded_args),
            &call.call_parameters,
            call.compute_calldata_offset,
            tx_parameters.gas_limit
        )
        .unwrap();

        let mut inputs: Vec<Input> = vec![];
        let mut outputs: Vec<Output> = vec![];

        let self_contract_input = Input::contract(
            UtxoId::new(Bytes32::zeroed(), 0),
            Bytes32::zeroed(),
            Bytes32::zeroed(),
            contract_id,
        );
        inputs.push(self_contract_input);

        let mut spendables = wallet
            .get_spendable_coins(&AssetId::default(), DEFAULT_SPENDABLE_COIN_AMOUNT as u64)
            .await
            .unwrap();

        // add default asset change if any inputs are being spent
        if !spendables.is_empty() {
            let change_output = Output::change(wallet.address(), 0, AssetId::default());
            outputs.push(change_output);
        }

        if call.call_parameters.asset_id != AssetId::default() {
            let alt_spendables = wallet
                .get_spendable_coins(&call.call_parameters.asset_id, call.call_parameters.amount)
                .await
                .unwrap();

            // add alt change if inputs are being spent
            if !alt_spendables.is_empty() {
                let change_output = Output::change(wallet.address(), 0, call.call_parameters.asset_id);
                outputs.push(change_output);
            }

            // add alt coins to inputs
            spendables.extend(alt_spendables.into_iter());
        }

        for coin in spendables {
            let input_coin = Input::coin_signed(
                UtxoId::from(coin.utxo_id),
                coin.owner.into(),
                coin.amount.0,
                coin.asset_id.into(),
                0,
                0,
            );

            inputs.push(input_coin);
        }

        let n_inputs = inputs.len();

        let self_contract_output = Output::contract(0, Bytes32::zeroed(), Bytes32::zeroed());
        outputs.push(self_contract_output);

        // Add external contract IDs to Input/Output pair, if applicable.
        if let Some(external_contract_ids) = call.external_contracts {
            for (idx, external_contract_id) in external_contract_ids.iter().enumerate() {
                // We must associate the right external contract input to the corresponding external
                // output index (TXO). We add the `n_inputs` offset because we added some inputs
                // above.
                let output_index: u8 = (idx + n_inputs) as u8;
                let zeroes = Bytes32::zeroed();
                let external_contract_input = Input::contract(
                    UtxoId::new(Bytes32::zeroed(), output_index),
                    zeroes,
                    zeroes,
                    *external_contract_id,
                );

                inputs.push(external_contract_input);

                let external_contract_output = Output::contract(output_index, zeroes, zeroes);

                outputs.push(external_contract_output);
            }
        }

        // Add outputs to the transaction.
        if let Some(v) = call.variable_outputs {
            outputs.extend(v);
        };

        let mut tx = Transaction::script(
            tx_parameters.gas_price,
            tx_parameters.gas_limit,
            tx_parameters.byte_price,
            call.maturity,
            script,
            script_data,
            inputs,
            outputs,
            vec![],
        );
        wallet.sign_transaction(&mut tx).await.unwrap();

        Script::new(tx)
    }

    /// Given the necessary arguments, create a script that will be submitted to the node to call
    /// the contract. The script is the actual opcodes used to call the contract, and the script
    /// data is for instance the function selector. (script, script_data) is returned as a tuple
    /// of hex-encoded value vectors
    fn build_script_contents(
        contract_id: &ContractId,
        encoded_selector: &Option<Selector>,
        encoded_args: &Option<Vec<u8>>,
        call_parameters: &CallParameters,
        compute_calldata_offset: bool,
        gas_limit: u64
    ) -> Result<(Vec<u8>, Vec<u8>), Error> {
        use fuel_gql_client::fuel_types;
        // Script to call the contract.
        // We use the Opcode to call a contract: `CALL` pointing at the
        // following registers;
        //
        // 0x10 Script data offset
        // 0x11 Gas price  TODO: https://github.com/FuelLabs/fuels-rs/issues/184
        // 0x12 Coin amount
        // 0x13 Asset ID
        //
        // Note that these are soft rules as we're picking this addresses simply because they
        // non-reserved register.
        let forward_data_offset = AssetId::LEN + WORD_SIZE;
        let (script, offset) = script_with_data_offset!(
            data_offset,
            vec![
                // Load call data to 0x10.
                Opcode::MOVI(0x10, data_offset + forward_data_offset as Immediate18),
                // Load gas forward to 0x11.
                Opcode::MOVI(0x11, gas_limit as Immediate18),
                // Load word into 0x12
                Opcode::MOVI(
                    0x12,
                    ((data_offset as usize) + AssetId::LEN) as Immediate18
                ),
                // Load the amount into 0x12
                Opcode::LW(0x12, 0x12, 0),
                // Load the asset id to use to 0x13.
                Opcode::MOVI(0x13, data_offset),
                // Call the transfer contract.
                Opcode::CALL(0x10, 0x12, 0x13, REG_CGAS),
                Opcode::RET(REG_ONE),
            ]
        );

        #[allow(clippy::iter_cloned_collect)]
        let script = script.iter().copied().collect::<Vec<u8>>();

        // `script_data` consists of:
        // 1. Asset ID to be forwarded
        // 2. Amount to be forwarded
        // 3. Contract ID (ContractID::LEN);
        // 4. Function selector (1 * WORD_SIZE);
        // 5. Calldata offset, if it has structs as input,
        // computed as `script_data_offset` + ContractId::LEN
        //                                  + 2 * WORD_SIZE;
        // 6. Encoded arguments.
        let mut script_data: Vec<u8> = vec![];

        script_data.extend(call_parameters.asset_id.to_vec());

        let amount = call_parameters.amount as Word;
        script_data.extend(amount.to_be_bytes());

        script_data.extend(contract_id.as_ref());

        if let Some(e) = encoded_selector {
            script_data.extend(e)
        }

        // If the method call takes custom inputs or has more than
        // one argument, we need to calculate the `call_data_offset`,
        // which points to where the data for the custom types start in the
        // transaction. If it doesn't take any custom inputs, this isn't necessary.
        if compute_calldata_offset {
            // Offset of the script data relative to the call data
            let call_data_offset =
                ((offset as usize) + forward_data_offset) + ContractId::LEN + 2 * WORD_SIZE;
            let call_data_offset = call_data_offset as Word;

            script_data.extend(&call_data_offset.to_be_bytes());
        }

        // Insert encoded arguments, if any
        if let Some(e) = encoded_args {
            script_data.extend(e)
        }
        Ok((script, script_data))
    }

    // Calling the contract executes the transaction, and is thus state-modifying
    pub async fn call(self, fuel_client: &FuelClient) -> Result<Vec<Receipt>, Error> {
        let tx_id = fuel_client.submit(&self.tx).await?.0.to_string();
        let receipts = fuel_client.receipts(&tx_id).await?;
        let status = fuel_client.transaction_status(&tx_id).await?;
        match status {
            TransactionStatus::Failure { reason, .. } => {
                Err(Error::ContractCallError(reason, receipts))
            }
            _ => Ok(receipts),
        }
    }

    // Simulating a call to the contract means that the actual state of the blockchain is not
    // modified, it is only simulated using a "dry-run".
    pub async fn simulate(self, fuel_client: &FuelClient) -> Result<Vec<Receipt>, Error> {
        let receipts = fuel_client.dry_run(&self.tx).await?;
        Ok(receipts)
    }
}
