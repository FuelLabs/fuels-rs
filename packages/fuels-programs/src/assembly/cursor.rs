use fuels_core::{error, types::errors::Result};

pub struct WasmFriendlyCursor<'a> {
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

    pub fn consume_fixed<const AMOUNT: usize>(
        &mut self,
        ctx: &'static str,
    ) -> Result<[u8; AMOUNT]> {
        let data = self
            .consume(AMOUNT, ctx)?
            .try_into()
            .expect("should have failed if not enough data");

        Ok(data)
    }

    pub fn consume_all(&mut self) -> &'a [u8] {
        let data = self.data;

        self.data = &[];

        data
    }

    pub fn unconsumed(&self) -> usize {
        self.data.len()
    }
}
