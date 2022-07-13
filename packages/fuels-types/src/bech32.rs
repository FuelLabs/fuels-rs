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

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_new() {
        let addr_data = [
            48, 101, 49, 52, 48, 102, 48, 55, 48, 100, 49, 97, 102, 117, 51, 57, 49, 50, 48, 54,
            48, 98, 48, 100, 48, 56, 49, 53, 48, 52, 49, 52,
        ];
        let expected_encoding = "fuel1xpjnzdpsvccrwvryx9skvafn8ycnyvpkxp3rqeps8qcn2vp5xy6qu7yyz7";

        let bech32_addr = Bech32Address::new(FUEL_BECH32_HRP, addr_data.clone());

        assert_eq!(bech32_addr.plain_address(), Address::new(addr_data));
        assert_eq!(bech32_addr.encoding, expected_encoding)
    }

    #[tokio::test]
    async fn test_from_str() {
        let addr_data = [
            48, 101, 49, 52, 48, 102, 48, 55, 48, 100, 49, 97, 102, 117, 51, 57, 49, 50, 48, 54,
            48, 98, 48, 100, 48, 56, 49, 53, 48, 52, 49, 52,
        ];

        let bech32_addr = Bech32Address::from_str(
            "fuel1xpjnzdpsvccrwvryx9skvafn8ycnyvpkxp3rqeps8qcn2vp5xy6qu7yyz7",
        )
        .unwrap();

        assert_eq!(bech32_addr.plain_address(), Address::new(addr_data));
    }
}
