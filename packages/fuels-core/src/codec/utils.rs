use crate::types::errors::{error, Result};
use strum_macros::Display;

pub(crate) struct CounterWithLimit {
    count: usize,
    max: usize,
    name: String,
    direction: CodecDirection,
}

#[derive(Display)]
pub(crate) enum CodecDirection {
    #[strum(serialize = "encoding")]
    Encoding,
    #[strum(serialize = "decoding")]
    Decoding,
}

impl CounterWithLimit {
    pub(crate) fn new(max: usize, name: impl Into<String>, direction: CodecDirection) -> Self {
        Self {
            count: 0,
            max,
            direction,
            name: name.into(),
        }
    }

    pub(crate) fn increase(&mut self) -> Result<()> {
        self.count += 1;
        if self.count > self.max {
            Err(error!(
                InvalidType,
                "{} limit ({}) reached while {}. Try increasing it.",
                self.name,
                self.max,
                self.direction.to_string()
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
