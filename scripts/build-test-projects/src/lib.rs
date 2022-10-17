use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub fn discover_projects(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .expect("failed to walk directory")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .flat_map(|path| {
            if dir_contains_forc_manifest(&path) {
                vec![path]
            } else {
                discover_projects(&path)
            }
        })
        .collect()
}

pub fn build_recursively(path: &Path) -> impl Iterator<Item = BuildResult> {
    discover_projects(path).into_iter().map(|path| {
        let output = std::process::Command::new("forc")
            .args(["build", "--path"])
            .arg(&path)
            .output()
            .expect("failed to run `forc build` for example project");

        let compilation_result = BuildOutput {
            path,
            stderr: String::from_utf8(output.stderr).expect("Forc output is not valid utf8"),
        };

        if output.status.success() {
            BuildResult::Success(compilation_result)
        } else {
            BuildResult::Failure(compilation_result)
        }
    })
}

// Check if the given directory contains `Forc.toml` at its root.
pub fn dir_contains_forc_manifest(path: &Path) -> bool {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.path().file_name().and_then(|s| s.to_str()) == Some("Forc.toml") {
                return true;
            }
        }
    }
    false
}

pub struct BuildOutput {
    path: PathBuf,
    stderr: String,
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

    pub fn display_forc_info(&mut self) -> Result<(), std::io::Error> {
        let output = std::process::Command::new("forc")
            .args(["--version"])
            .output()?;

        let version =
            String::from_utf8(output.stdout).expect("failed to parse forc --version output");

        self.write_success_bold("\nBuilding ")?;
        self.write(&format!("projects with: `{}`\n\n", version.trim()))
    }

    pub fn display_result(
        &mut self,
        abs_path: &PathBuf,
        build_result: &BuildResult,
    ) -> Result<(), std::io::Error> {
        match build_result {
            BuildResult::Success(build_output) => {
                self.write(&format!(
                    "build {} ... ",
                    build_output.get_display_path(abs_path).display()
                ))?;
                self.write_success("ok\n")
            }
            BuildResult::Failure(build_output) => {
                self.write(&format!(
                    "build {} ... ",
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
