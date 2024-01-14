use crate::types::errors::{error, Result};

pub(crate) struct CounterWithLimit {
    count: usize,
    max: usize,
    name: String,
    decoding: bool,
}

impl CounterWithLimit {
    pub(crate) fn new(max: usize, name: impl Into<String>, decoding: bool) -> Self {
        Self {
            count: 0,
            max,
            decoding,
            name: name.into(),
        }
    }

    pub(crate) fn increase(&mut self) -> Result<()> {
        self.count += 1;
        if self.count > self.max {
            let direction = if self.decoding {
                "decoding"
            } else {
                "encoding"
            };
            Err(error!(
                InvalidType,
                "{} limit ({}) reached while {}. Try increasing it.",
                self.name,
                self.max,
                direction
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn decrease(&mut self) {
        if self.count > 0 {
            self.count -= 1;
        }
    }
}
