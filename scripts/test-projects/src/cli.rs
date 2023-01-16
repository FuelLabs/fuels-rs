use std::path::PathBuf;

use clap::{command, Parser, Subcommand};

const TESTS_PATH: &str = "packages/fuels/tests/";
const NUM_CONCURRENT: usize = 8;

#[derive(Parser)]
#[command(name = "test-projects", version, about, propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommand,

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
pub enum CliCommand {
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

pub struct RunConfig {
    pub project_path: PathBuf,
    pub bin_path: PathBuf,
    pub prepared_command: PreparedCommand,
    pub num_concurrent: usize,
}

pub struct PreparedCommand {
    pub command: String,
    pub args: Vec<String>,
    pub info: String,
}

pub fn parse_cli() -> RunConfig {
    let cli = Cli::parse();

    let project_path = cli
        .projects_path
        .unwrap_or_else(|| PathBuf::from(TESTS_PATH));
    let project_path = project_path.canonicalize().unwrap_or_else(|_| {
        panic!(
            "project path
            {:?} could not be canonicalized",
            project_path
        )
    });

    let bin_path = if let Some(bin_path) = cli.bin_path {
        bin_path
            .canonicalize()
            .unwrap_or_else(|_| panic!("bin path {:?} could not be canonicalized", bin_path))
    } else {
        PathBuf::from("")
    };

    let prepared_command = match cli.command {
        CliCommand::Clean => PreparedCommand {
            command: bin_path.join("forc").display().to_string(),
            args: vec!["clean".into(), "--path".into()],
            info: "clean".into(),
        },
        CliCommand::Build => PreparedCommand {
            command: bin_path.join("forc").display().to_string(),
            args: vec!["build".into(), "--path".into()],
            info: "build".into(),
        },
        CliCommand::Format { check } => {
            if check {
                PreparedCommand {
                    command: bin_path.join("forc-fmt").display().to_string(),
                    args: vec!["--check".into(), "--path".into()],
                    info: "check".into(),
                }
            } else {
                PreparedCommand {
                    command: bin_path.join("forc-fmt").display().to_string(),
                    args: vec!["--path".into()],
                    info: "format".into(),
                }
            }
        }
    };

    RunConfig {
        num_concurrent: cli.num_concurrent,
        project_path,
        bin_path,
        prepared_command,
    }
}
