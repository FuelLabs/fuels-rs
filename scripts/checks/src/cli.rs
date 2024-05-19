use std::path::PathBuf;

use clap::{arg, Parser, ValueEnum};

#[derive(Parser)]
#[command(about = "Runs pre-release checks deemed to heavy for CI.")]
pub struct Cli {
    ///Comma separated list of tasks to run
    #[arg(
        short,
        long,
        value_delimiter = ',',
        num_args = 0..

    )]
    pub only_tasks_with_ids: Option<Vec<String>>,

    /// Prints out all tasks available (depends on what `flavor` is enabled)
    #[arg(short, long, action)]
    pub list_tasks: bool,

    /// Print json job description to be used for the CI
    #[arg(long)]
    pub print_ci_jobs_desc: bool,

    /// Only run tasks in the given directories
    #[arg(
        long,
        value_delimiter = ',',
        num_args = 0..

    )]
    pub only_tasks_in_dir: Option<Vec<PathBuf>>,

    /// Used to enable/disable tests that take too long/are too resource intense.
    #[arg(short, long, default_value = "normal")]
    pub flavor: Flavor,

    /// Disables tests that need the sway artifacts to be built with the type paths enabled.
    /// Enabled by default.
    #[arg(short, long, default_value_t = true)]
    pub disable_type_paths: bool,

    /// Enable verbose output.
    #[arg(short, long, default_value = "false")]
    pub verbose: bool,

    /// If ran as a binary from elsewhere the workspace_root needs to be pointed to where the
    /// project workspace root is
    #[arg(short, long = "root", required = true)]
    pub root: PathBuf,
}

#[derive(Debug, Copy, Clone, ValueEnum, PartialEq)]
pub enum Flavor {
    Normal,
    HackFeatures,
    HackDeps,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tasks_can_be_selected() {
        // given
        let cli = "foo --only-tasks-with-ids one,two -r .";

        // when
        let cli = Cli::try_parse_from(cli.split_whitespace()).unwrap();

        // then
        assert_eq!(
            cli.only_tasks_with_ids,
            Some(vec!["one".to_string(), "two".to_string()])
        );
    }

    #[test]
    fn tasks_can_be_listed() {
        // given
        let cli = "foo --list-tasks -r .";

        // when
        let cli = Cli::try_parse_from(cli.split_whitespace()).unwrap();

        // then
        assert!(cli.list_tasks);
    }

    #[test]
    fn flavor_can_be_chosen() {
        // given
        let cli = "foo --flavor hack-features -r .";

        // when
        let cli = Cli::try_parse_from(cli.split_whitespace()).unwrap();

        // then
        assert_eq!(cli.flavor, Flavor::HackFeatures);
    }

    #[test]
    fn default_flavor_is_normal() {
        // given
        let cli = "foo -r .";

        // when
        let cli = Cli::try_parse_from(cli.split_whitespace()).unwrap();

        // then
        assert_eq!(cli.flavor, Flavor::Normal);
    }
}
