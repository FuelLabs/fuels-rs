use std::{collections::HashMap, fmt::Debug};

use fuel_asm::Word;
use fuel_tx::{AssetId, ContractId};
use fuels_core::{
    constants::{DEFAULT_CALL_PARAMS_AMOUNT, WORD_SIZE},
    error,
    types::{
        bech32::{Bech32Address, Bech32ContractId},
        errors::Result,
        param_types::ParamType,
        Selector,
    },
};

use crate::calls::utils::sealed;

use super::utils::CallOpcodeParamsOffset;

pub(crate) struct WasmFriendlyCursor<'a> {
    data: &'a [u8],
}

impl<'a> WasmFriendlyCursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn consume(&mut self, amount: usize, ctx: &'static str) -> Result<&'a [u8]> {
        if self.data.len() < amount {
            Err(error!(
                Other,
                "while decoding {ctx}: not enough data, available: {}, requested: {}",
                self.data.len(),
                amount
            ))
        } else {
            let data = &self.data[..amount];
            self.data = &self.data[amount..];
            Ok(data)
        }
    }

    pub fn consume_all(&self) -> &'a [u8] {
        self.data
    }

    pub fn unconsumed(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug, Clone)]
/// Contains all data relevant to a single contract call
pub struct ContractCall {
    pub contract_id: Bech32ContractId,
    pub encoded_args: Result<Vec<u8>>,
    pub encoded_selector: Selector,
    pub call_parameters: CallParameters,
    pub external_contracts: Vec<Bech32ContractId>,
    pub output_param: ParamType,
    pub is_payable: bool,
    pub custom_assets: HashMap<(AssetId, Option<Bech32Address>), u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractCallData {
    pub amount: u64,
    pub asset_id: AssetId,
    pub contract_id: ContractId,
    pub fn_selector_encoded: Vec<u8>,
    pub encoded_args: Vec<u8>,
    pub gas_forwarded: Option<u64>,
}

impl ContractCallData {
    pub fn decode_fn_selector(&self) -> Result<String> {
        String::from_utf8(self.fn_selector_encoded.clone())
            .map_err(|e| error!(Codec, "cannot decode function selector: {}", e))
    }

    /// Encodes as script data, consisting of the following items in the given order:
    /// 1. Amount to be forwarded `(1 * `[`WORD_SIZE`]`)`
    /// 2. Asset ID to be forwarded ([`AssetId::LEN`])
    /// 3. Contract ID ([`ContractId::LEN`]);
    /// 4. Function selector offset `(1 * `[`WORD_SIZE`]`)`
    /// 5. Calldata offset `(1 * `[`WORD_SIZE`]`)`
    /// 6. Encoded function selector - method name
    /// 7. Encoded arguments
    /// 8. Gas to be forwarded `(1 * `[`WORD_SIZE`]`)` - Optional
    pub(crate) fn encode(
        &self,
        memory_offset: usize,
        buffer: &mut Vec<u8>,
    ) -> CallOpcodeParamsOffset {
        let amount_offset = memory_offset;
        let asset_id_offset = amount_offset + WORD_SIZE;
        let call_data_offset = asset_id_offset + AssetId::LEN;
        let encoded_selector_offset = call_data_offset + ContractId::LEN + 2 * WORD_SIZE;
        let encoded_args_offset = encoded_selector_offset + self.fn_selector_encoded.len();

        buffer.extend(self.amount.to_be_bytes()); // 1. Amount

        let asset_id = self.asset_id;
        buffer.extend(asset_id.iter()); // 2. Asset ID

        buffer.extend(self.contract_id.as_ref()); // 3. Contract ID

        buffer.extend((encoded_selector_offset as Word).to_be_bytes()); // 4. Fun. selector offset

        buffer.extend((encoded_args_offset as Word).to_be_bytes()); // 5. Calldata offset

        buffer.extend(&self.fn_selector_encoded); // 6. Encoded function selector

        let encoded_args_len = self.encoded_args.len();

        buffer.extend(&self.encoded_args); // 7. Encoded arguments

        let gas_forwarded_offset = self.gas_forwarded.map(|gf| {
            buffer.extend((gf as Word).to_be_bytes()); // 8. Gas to be forwarded - Optional

            encoded_args_offset + encoded_args_len
        });

        CallOpcodeParamsOffset {
            amount_offset,
            asset_id_offset,
            gas_forwarded_offset,
            call_data_offset,
        }
    }

    pub fn decode(data: &[u8], gas_fwd: bool) -> Result<Self> {
        let mut data = WasmFriendlyCursor::new(data);

        let amount = u64::from_be_bytes(
            data.consume(8, "amount")?
                .try_into()
                .expect("will have exactly 8 bytes"),
        );

        let asset_id = AssetId::new(
            data.consume(32, "asset id")?
                .try_into()
                .expect("will have exactly 32 bytes"),
        );

        let contract_id = ContractId::new(
            data.consume(32, "contract id")?
                .try_into()
                .expect("will have exactly 32 bytes"),
        );

        let _ = data.consume(8, "function selector offset")?;

        let _ = data.consume(8, "encoded args offset")?;

        let fn_selector = {
            let fn_selector_len = {
                let bytes = data.consume(8, "function selector lenght")?;
                u64::from_be_bytes(bytes.try_into().expect("will have exactly 8 bytes")) as usize
            };
            data.consume(fn_selector_len, "function selector")?.to_vec()
        };

        let (encoded_args, gas_forwarded) = if gas_fwd {
            let encoded_args = data
                .consume(data.unconsumed().saturating_sub(WORD_SIZE), "encoded_args")?
                .to_vec();

            let gas_fwd = {
                let gas_fwd_bytes = data.consume(WORD_SIZE, "forwarded gas")?;
                u64::from_be_bytes(gas_fwd_bytes.try_into().expect("exactly 8 bytes"))
            };

            (encoded_args, Some(gas_fwd))
        } else {
            (data.consume_all().to_vec(), None)
        };

        Ok(ContractCallData {
            amount,
            asset_id,
            contract_id,
            fn_selector_encoded: fn_selector,
            encoded_args,
            gas_forwarded,
        })
    }
}

impl ContractCall {
    pub fn data(&self, base_asset_id: AssetId) -> Result<ContractCallData> {
        let encoded_args = self
            .encoded_args
            .as_ref()
            .map_err(|e| error!(Codec, "cannot encode contract call arguments: {e}"))?
            .to_owned();

        Ok(ContractCallData {
            amount: self.call_parameters.amount(),
            asset_id: self.call_parameters.asset_id().unwrap_or(base_asset_id),
            contract_id: self.contract_id.clone().into(),
            fn_selector_encoded: self.encoded_selector.clone(),
            encoded_args,
            gas_forwarded: self.call_parameters.gas_forwarded,
        })
    }

    pub fn with_contract_id(self, contract_id: Bech32ContractId) -> Self {
        ContractCall {
            contract_id,
            ..self
        }
    }

    pub fn with_call_parameters(self, call_parameters: CallParameters) -> ContractCall {
        ContractCall {
            call_parameters,
            ..self
        }
    }

    pub fn add_custom_asset(&mut self, asset_id: AssetId, amount: u64, to: Option<Bech32Address>) {
        *self.custom_assets.entry((asset_id, to)).or_default() += amount;
    }
}

impl sealed::Sealed for ContractCall {}

#[derive(Debug, Clone)]
pub struct CallParameters {
    amount: u64,
    asset_id: Option<AssetId>,
    gas_forwarded: Option<u64>,
}

impl CallParameters {
    pub fn new(amount: u64, asset_id: AssetId, gas_forwarded: u64) -> Self {
        Self {
            amount,
            asset_id: Some(asset_id),
            gas_forwarded: Some(gas_forwarded),
        }
    }

    pub fn with_amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn with_asset_id(mut self, asset_id: AssetId) -> Self {
        self.asset_id = Some(asset_id);
        self
    }

    pub fn asset_id(&self) -> Option<AssetId> {
        self.asset_id
    }

    pub fn with_gas_forwarded(mut self, gas_forwarded: u64) -> Self {
        self.gas_forwarded = Some(gas_forwarded);
        self
    }

    pub fn gas_forwarded(&self) -> Option<u64> {
        self.gas_forwarded
    }
}

impl Default for CallParameters {
    fn default() -> Self {
        Self {
            amount: DEFAULT_CALL_PARAMS_AMOUNT,
            asset_id: None,
            gas_forwarded: None,
        }
    }
}
