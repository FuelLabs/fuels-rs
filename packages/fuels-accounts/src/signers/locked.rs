use fuels_core::{traits::AddressResolver, types::bech32::Bech32Address};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locked {
    address: Bech32Address,
}

impl Locked {
    pub fn new(address: Bech32Address) -> Self {
        Self { address }
    }
}

impl AddressResolver for Locked {
    fn address(&self) -> &Bech32Address {
        &self.address
    }
}
