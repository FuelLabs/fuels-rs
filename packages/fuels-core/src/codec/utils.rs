use crate::types::errors::{error, Result};

pub(crate) struct CounterWithLimit {
    count: usize,
    max: usize,
    name: String,
}

impl CounterWithLimit {
    pub(crate) fn new(max: usize, name: impl Into<String>) -> Self {
        Self {
            count: 0,
            max,
            name: name.into(),
        }
    }

    pub(crate) fn increase(&mut self) -> Result<()> {
        self.count += 1;
        if self.count > self.max {
            Err(error!(
                InvalidType,
                "{} limit ({}) reached while decoding. Try increasing it.", self.name, self.max
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
