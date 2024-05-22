use crate::custom_checks;

use super::{
    command::Command,
    report::{Report, Status},
    short_sha256,
};

use std::fmt::Display;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Task {
    pub cwd: PathBuf,
    pub cmd: Command,
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task {}, dir: {:?}, {}", self.id(), self.cwd, self.cmd)
    }
}

impl Task {
    pub fn id(&self) -> String {
        short_sha256(&format!("{self:?}"))
    }

    pub fn run(self) -> Report {
        match &self.cmd {
            Command::Custom {
                program, args, env, ..
            } => self.run_custom(program, args.iter().map(String::as_str), env),
            Command::MdCheck => self.run_md_check(),
            Command::VerifyCoreVersion => self.run_verify_core_version(),
        }
    }

    fn run_verify_core_version(&self) -> Report {
        let status =
            if let Err(e) = custom_checks::fuel_core_version::verify_core_version(&self.cwd) {
                e.into()
            } else {
                Status::Success { out: String::new() }
            };
        self.report(status)
    }

    fn run_md_check(&self) -> Report {
        let status = if let Err(e) = custom_checks::md_check::run(&self.cwd) {
            e.into()
        } else {
            Status::Success { out: String::new() }
        };

        self.report(status)
    }

    fn run_custom<'a, F>(&self, program: &str, args: F, env: &[(String, String)]) -> Report
    where
        F: IntoIterator<Item = &'a str>,
    {
        let mut cmd = duct::cmd(program, args)
            .stderr_to_stdout()
            .dir(&self.cwd)
            .stdin_null()
            .stdout_capture()
            .unchecked();

        for (key, value) in env {
            cmd = cmd.env(key, value);
        }

        let output = match cmd.run() {
            Ok(output) => output,
            Err(err) => return self.report(err),
        };

        let decoded = String::from_utf8_lossy(&output.stdout).into_owned();
        let status = if output.status.success() {
            Status::Success { out: decoded }
        } else {
            Status::Failed { reason: decoded }
        };

        self.report(status)
    }

    pub(crate) fn report(&self, status: impl Into<Status>) -> Report {
        Report {
            cmd_desc: self.to_string(),
            status: status.into(),
        }
    }
}
