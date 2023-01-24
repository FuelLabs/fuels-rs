use fuel_tx::{Address, ContractId};
use fuels_macros::{Parameterize, Tokenizable};
use serde::{Deserialize, Serialize};

use crate::{
    core::Token,
    enum_variants::EnumVariants,
    errors::Error,
    param_types::ParamType,
    traits::{Parameterize, Tokenizable},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Parameterize, Tokenizable)]
pub enum Identity {
    Address(Address),
    ContractId(ContractId),
}
