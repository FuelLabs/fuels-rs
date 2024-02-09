use crate::types::errors::{error, Result};

pub(crate) struct CounterWithLimit {
    count: usize,
    max: usize,
    name: String,
    direction: CodecDirection,
}

#[derive(Debug)]
pub(crate) enum CodecDirection {
    Encoding,
    Decoding,
}

impl std::fmt::Display for CodecDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecDirection::Encoding => write!(f, "encoding"),
            CodecDirection::Decoding => write!(f, "decoding"),
        }
    }
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
            return Err(error!(
                InvalidType,
                "{} limit ({}) reached while {}. Try increasing it.",
                self.name,
                self.max,
                self.direction
            ));
        }
        Ok(())
    }

    pub(crate) fn decrease(&mut self) {
        if self.count > 0 {
            self.count -= 1;
        }
    }
}
