#![allow(clippy::assign_op_pattern)]

use fuel_tx::{Address, ContractId};
use fuels_macros::{Parameterize, Tokenizable, TryFrom};
use impl_serde::impl_uint_serde;
use serde::{Deserialize, Serialize};
use uint::construct_uint;

use crate::{
    traits::{Parameterize, Tokenizable},
    types::{
        errors::{error, Error, Result as FuelsResult},
        param_types::ParamType,
        Token,
    },
};

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}
impl_uint_serde!(U256, 4);

impl Parameterize for U256 {
    fn param_type() -> ParamType {
        ParamType::U256
    }
}

impl Tokenizable for U256 {
    fn from_token(token: Token) -> FuelsResult<Self>
    where
        Self: Sized,
    {
        match token {
            Token::U256(data) => Ok(data),
            _ => Err(error!(
                InvalidData,
                "U256 cannot be constructed from token {token}"
            )),
        }
    }

    fn into_token(self) -> Token {
        Token::U256(self)
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Parameterize, Tokenizable, TryFrom,
)]
#[FuelsCorePath = "crate"]
#[FuelsTypesPath = "crate::types"]
pub enum Identity {
    Address(Address),
    ContractId(ContractId),
}

impl Default for Identity {
    fn default() -> Self {
        Self::Address(Address::default())
    }
}

impl AsRef<[u8]> for Identity {
    fn as_ref(&self) -> &[u8] {
        match self {
            Identity::Address(address) => address.as_ref(),
            Identity::ContractId(contract_id) => contract_id.as_ref(),
        }
    }
}
