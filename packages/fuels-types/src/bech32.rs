use bech32::{u5, Variant, FromBase32, ToBase32};
use core::str;
use fuel_tx::Address;

pub struct Bech32Address {
    hrp: String,
    data: [u8; 32],
    variant: Variant,
    encoding: String,
}

impl Bech32Address {
    pub fn new(hrp: &str, data: [u8; 32], variant: Variant) -> Self {
        let data_base32 = data.to_base32();
        let encoding = bech32::encode(hrp, &data_base32, variant).unwrap();

        Self {
            hrp: hrp.to_string(),
            data,
            variant,
            encoding,
        }
    }

    pub fn len(&self) -> usize {
        self.encoding.len()
    }

    pub fn variant(&self) -> Variant {
        self.variant
    }
}

impl Into<Address> for Bech32Address {
    fn into(self) -> Address {
        Address::new(self.data)
    }
}

impl str::FromStr for Bech32Address {
    type Err = bech32::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (hrp, data, variant) = bech32::decode(s)?;

        Ok(Self {
            hrp,
            data,
            variant,
            encoding: s.to_string(),
        })
    }
}
