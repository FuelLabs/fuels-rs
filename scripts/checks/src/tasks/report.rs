use colored::Colorize;

#[derive(Debug, Clone)]
pub struct Report {
    pub cmd_desc: String,
    pub status: Status,
}

impl From<std::io::Error> for Status {
    fn from(value: std::io::Error) -> Self {
        Self::Failed {
            reason: value.to_string(),
        }
    }
}

impl From<anyhow::Error> for Status {
    fn from(value: anyhow::Error) -> Self {
        Self::Failed {
            reason: value.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Status {
    Success { out: String },
    Failed { reason: String },
}

impl Report {
    pub fn report(&self, tty: bool, verbose: bool) -> String {
        let status = match &self.status {
            Status::Failed { reason } => {
                let err = if tty { "error".red() } else { "error".normal() };
                format!("{err}\n{reason}")
            }
            Status::Success { out } => {
                let ok = if tty { "ok".green() } else { "ok".normal() };
                if verbose {
                    format!("{ok}\n{out}")
                } else {
                    ok.to_string()
                }
            }
        };

        format!("{} ... {status}", self.cmd_desc)
    }
}
