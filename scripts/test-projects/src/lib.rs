use clap::{command, Parser, Subcommand};
use futures_util::{stream, Stream, StreamExt};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

const NUM_CONCURRENT: usize = 1;

#[derive(Parser)]
#[command(name = "test-projects", version, about, propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Number of concurrent projects
    #[arg(short, long, default_value_t = NUM_CONCURRENT)]
    #[arg(value_name = "NUM")]
    pub num_concurrent: usize,

    /// Specify where to find `forc` and `forc-fmt`
    #[arg(long, value_name = "DIR")]
    pub bin_path: Option<PathBuf>,

    /// Specify test projects path
    #[arg(long, value_name = "DIR")]
    pub projects_path: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Cleans `forc` build output
    Clean,
    /// Builds all test projects with `forc`
    Build,
    /// Formats all test projects with `forc-fmt`
    Format {
        /// Checks format but doesn't modify files
        #[arg(long)]
        check: bool,
    },
}

pub struct Command2Run {
    pub command: String,
    pub args: Vec<String>,
    pub info: String,
}

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

pub fn run_recursively(
    path: &Path,
    num_conc_futures: usize,
    command: String,
    args: Vec<String>,
) -> impl Stream<Item = BuildResult> {
    stream::iter(discover_projects(path))
        .map(move |path| {
            let command = command.clone();
            let args = args.clone();

            async move {
                let output = tokio::process::Command::new(command)
                    .args(args)
                    .arg(&path)
                    .output()
                    .await
                    .expect("failed to run command");

                let compilation_result = BuildOutput {
                    path,
                    stderr: String::from_utf8(output.stderr).expect("stderr is not valid utf8"),
                };

                if output.status.success() {
                    BuildResult::Success(compilation_result)
                } else {
                    BuildResult::Failure(compilation_result)
                }
            }
        })
        .buffer_unordered(num_conc_futures)
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

    pub fn display_info(
        &mut self,
        num_conc_futures: usize,
        command: &str,
        run_info: &str,
    ) -> Result<(), std::io::Error> {
        let output = std::process::Command::new(command)
            .args(["--version"])
            .output()
            .unwrap_or_else(|err| panic!("failed to run `{command} --version`. {err}"));

        let version = String::from_utf8(output.stdout)
            .unwrap_or_else(|_| panic!("failed to parse `{command} --version` output"));

        self.write_success_bold(&format!("\n{:>7} ", run_info))?;
        self.write(&format!(
            "projects with: `{}`\n        num concurrent projects: {}\n\n",
            version.trim(),
            num_conc_futures
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
