use std::{
    io::Write,
    path::{Path, PathBuf},
};

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::cli::RunConfig;

pub struct BuildOutput {
    pub path: PathBuf,
    pub stderr: String,
}

pub enum BuildResult {
    Success(BuildOutput),
    Failure(BuildOutput),
}

pub struct ResultWriter {
    stdout: StandardStream,
    green: ColorSpec,
    red: ColorSpec,
}

impl BuildOutput {
    fn get_display_path(&self, abs_path: &PathBuf) -> &Path {
        self.path
            .strip_prefix(abs_path)
            .expect("Could not strip path prefix")
    }
}

impl ResultWriter {
    pub fn new() -> Self {
        Self {
            stdout: StandardStream::stdout(ColorChoice::Always),
            green: ColorSpec::new().set_fg(Some(Color::Green)).to_owned(),
            red: ColorSpec::new().set_fg(Some(Color::Red)).to_owned(),
        }
    }

    fn write(&mut self, text: &str) -> Result<(), std::io::Error> {
        write!(&mut self.stdout, "{}", text)
    }

    fn write_success(&mut self, text: &str) -> Result<(), std::io::Error> {
        self.stdout.set_color(&self.green)?;
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.reset()
    }

    fn write_success_bold(&mut self, text: &str) -> Result<(), std::io::Error> {
        self.stdout.set_color(self.green.clone().set_bold(true))?;
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.reset()
    }

    fn write_error(&mut self, text: &str) -> Result<(), std::io::Error> {
        self.stdout.set_color(&self.red)?;
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.reset()
    }

    pub fn display_info(&mut self, config: &RunConfig) -> Result<(), std::io::Error> {
        let command = &config.prepared_command.command;
        let output = std::process::Command::new(command)
            .args(["--version"])
            .output()
            .unwrap_or_else(|err| panic!("failed to run `{command} --version`. {err}"));

        let version = String::from_utf8(output.stdout)
            .unwrap_or_else(|_| panic!("failed to parse `{command} --version` output"));

        self.write_success_bold(&format!("\n{:>7} ", &config.prepared_command.info))?;
        self.write(&format!(
            "projects with: `{}`\n        num concurrent projects: {}\n\n",
            version.trim(),
            config.num_concurrent
        ))
    }

    pub fn display_result(
        &mut self,
        abs_path: &PathBuf,
        build_result: &BuildResult,
        result_info: &str,
    ) -> Result<(), std::io::Error> {
        match build_result {
            BuildResult::Success(build_output) => {
                self.write(&format!(
                    "{} {} ... ",
                    result_info,
                    build_output.get_display_path(abs_path).display()
                ))?;
                self.write_success("ok\n")
            }
            BuildResult::Failure(build_output) => {
                self.write(&format!(
                    "{} {} ... ",
                    result_info,
                    build_output.get_display_path(abs_path).display()
                ))?;
                self.write_error("FAILED\n")
            }
        }
    }

    pub fn display_failed(
        &mut self,
        abs_path: &PathBuf,
        failed: &[BuildResult],
    ) -> Result<(), std::io::Error> {
        self.write("\nfailures:\n\n")?;
        for f in failed {
            self.display_error(abs_path, f)?;
        }

        self.write("\nfailures:\n")?;
        for f in failed {
            self.display_failed_project(abs_path, f)?;
        }
        Ok(())
    }

    fn display_error(
        &mut self,
        abs_path: &PathBuf,
        build_result: &BuildResult,
    ) -> Result<(), std::io::Error> {
        if let BuildResult::Failure(build_output) = build_result {
            self.write(&format!(
                "---- {} ----\n{}\n",
                build_output.get_display_path(abs_path).display(),
                build_output.stderr
            ))?;
        }
        Ok(())
    }

    fn display_failed_project(
        &mut self,
        abs_path: &PathBuf,
        build_result: &BuildResult,
    ) -> Result<(), std::io::Error> {
        if let BuildResult::Failure(build_output) = build_result {
            self.write(&format!(
                "    {}\n",
                build_output.get_display_path(abs_path).display(),
            ))?
        }
        Ok(())
    }

    pub fn display_stats(
        &mut self,
        num_succeeded: usize,
        num_failed: usize,
    ) -> Result<(), std::io::Error> {
        self.write("\nbuild result: ")?;

        if num_failed > 0 {
            self.write_error("FAILED")?;
        } else {
            self.write_success("ok")?;
        };

        self.write(&format!(
            ". {} passed, {} failed\n",
            num_succeeded, num_failed
        ))
    }
}

impl Default for ResultWriter {
    fn default() -> Self {
        Self::new()
    }
}
