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
    pub only_tasks_with_ids: Vec<String>,

    /// Prints out all tasks available (depends on what `flavor` is enabled)
    #[arg(short, long, action)]
    pub list_tasks: bool,

    /// List directories that are used by the tasks in JSON format.
    #[arg(long)]
    pub list_used_dirs: bool,

    /// Only run tasks in the given directory
    #[arg(
        long,
        value_delimiter = ',',
        num_args = 0..

    )]
    pub only_tasks_in_dir: Vec<String>,

    /// Used to enable/disable tests that take too long/are too resource intense.
    #[arg(short, long, default_value = "ci")]
    pub flavor: Flavor,

    /// Sway project compiled with type path support
    #[arg(short, long, action)]
    pub sway_with_type_paths: bool,

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
    Ci,
    Max,
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
        assert_eq!(cli.only_tasks_with_ids, vec!["one", "two"]);
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
        let cli = "foo --flavor max -r .";

        // when
        let cli = Cli::try_parse_from(cli.split_whitespace()).unwrap();

        // then
        assert_eq!(cli.flavor, Flavor::Max);
    }

    #[test]
    fn default_flavor_is_ci() {
        // given
        let cli = "foo -r .";

        // when
        let cli = Cli::try_parse_from(cli.split_whitespace()).unwrap();

        // then
        assert_eq!(cli.flavor, Flavor::Ci);
    }
}
