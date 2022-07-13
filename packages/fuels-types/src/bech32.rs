use crate::errors::Error;
use bech32::Variant::Bech32m;
use bech32::{FromBase32, ToBase32};
use core::str;
use fuel_tx::Address;
use std::fmt::{Display, Formatter};

pub const FUEL_BECH32_HRP: &str = "fuel";

#[derive(Debug, Clone)]
pub struct Bech32Address {
    plain_addr: Address,
    encoding: String,
}

impl Bech32Address {
    pub fn new(hrp: &str, data: [u8; 32]) -> Self {
        let data_base32 = data.to_base32();
        let encoding = bech32::encode(hrp, &data_base32, Bech32m).unwrap();

        Self {
            plain_addr: Address::from(data),
            encoding,
        }
    }

    /// Returns the plain address string
    pub fn plain_address(&self) -> Address {
        self.plain_addr
    }

    /// Returns the plain address string
    pub fn to_plain_addr_str(&self) -> String {
        self.plain_addr.to_string()
    }
}

#[allow(clippy::from_over_into)]
impl Into<Address> for &Bech32Address {
    fn into(self) -> Address {
        self.plain_addr
    }
}

impl str::FromStr for Bech32Address {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (_, data_base32, _) = bech32::decode(s)?;

        let data: [u8; Address::LEN] = Vec::<u8>::from_base32(&data_base32)
            .unwrap()
            .as_slice()
            .try_into()?;

        Ok(Self {
            plain_addr: Address::from(data),
            encoding: s.to_string(),
        })
    }
}

impl Display for Bech32Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.encoding)
    }
}
