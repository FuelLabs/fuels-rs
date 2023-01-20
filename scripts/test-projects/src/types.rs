use std::process::Output;
use std::{io::Write, path::PathBuf};

use serde::Deserialize;
use serde::Serialize;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::cli::RunConfig;

#[derive(Deserialize, Serialize)]
pub struct Manifest {
    pub workspace: Workspace,
}
#[derive(Deserialize, Serialize)]
pub struct Workspace {
    pub members: Vec<PathBuf>,
}

pub struct ResultWriter {
    stdout: StandardStream,
    green: ColorSpec,
    yellow: ColorSpec,
}

impl ResultWriter {
    pub fn new() -> Self {
        Self {
            stdout: StandardStream::stdout(ColorChoice::Always),
            green: ColorSpec::new().set_fg(Some(Color::Green)).to_owned(),
            yellow: ColorSpec::new().set_fg(Some(Color::Yellow)).to_owned(),
        }
    }

    fn write(&mut self, text: &str) -> Result<(), std::io::Error> {
        write!(&mut self.stdout, "{}", text)
    }

    fn write_success_bold(&mut self, text: &str) -> Result<(), std::io::Error> {
        self.stdout.set_color(self.green.clone().set_bold(true))?;
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.reset()
    }

    fn write_warning(&mut self, text: &str) -> Result<(), std::io::Error> {
        self.stdout.set_color(self.yellow.clone().set_bold(true))?;
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.reset()
    }

    pub fn display_warning(&mut self, text: &str) -> Result<(), std::io::Error> {
        self.write_warning(text)
    }

    pub fn display_info(
        &mut self,
        config: &RunConfig,
        output_result: Output,
    ) -> Result<(), std::io::Error> {
        let command = &config.prepared_command.command;
        let output = std::process::Command::new(command)
            .args(["--version"])
            .output()
            .map_err(|err| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to run `{} --version`. {}", command, err),
                )
            })?;

        let version = String::from_utf8(output.stdout).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to parse `{} --version` output",
            )
        })?;

        self.write_success_bold(&format!("\n{:>7} ", &config.prepared_command.info))?;

        self.write(&format!("projects workspace with: `{}`\n", version.trim(),))?;

        let output_err = String::from_utf8(output_result.stderr.clone()).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to parse `{} --version` output",
            )
        })?;

        self.write(&format!("{}\n", output_err,))?;

        if output_result.status.success() {
            self.write_success_bold(
                format!(
                    "{:>7}{}\n",
                    &config.prepared_command.info, "ing was successful!"
                )
                .as_str(),
            )?
        }
        Ok(())
    }
}

impl Default for ResultWriter {
    fn default() -> Self {
        Self::new()
    }
}
