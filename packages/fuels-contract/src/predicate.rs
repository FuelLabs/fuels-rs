use fuel_gql_client::fuel_tx::{Address, Contract};
use fuels_types::errors::Error;

use fuels_types::bech32::Bech32Address;

pub struct Predicate {
    address: Bech32Address,
    code: Vec<u8>,
}

impl Predicate {
    pub fn new(code: Vec<u8>) -> Self {
        let address: Address = (*Contract::root_from_code(&code)).into();
        Self {
            address: address.into(),
            code,
        }
    }

    pub fn load_from(file_path: &str) -> Result<Self, Error> {
        Ok(Predicate::new(std::fs::read(file_path)?))
    }

    pub fn address(&self) -> &Bech32Address {
        &self.address
    }

    pub fn code(&self) -> Vec<u8> {
        self.code.clone()
    }
}
