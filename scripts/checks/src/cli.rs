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
    #[arg(default_value_t = default_workspace_root())]
    pub workspace_root: String,
}

fn default_workspace_root() -> String {
    // This will bake in the current path of the project as the default workspace_root. Makes it so that the checks can be run from anywhere in the project.
    // Would usually be a bad idea, but in this case it's fine since it's utility code.
    PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("../../../")
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .to_string()
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
        let cli = "foo --tasks one,two";

        // when
        let cli = Cli::parse_from(cli.split_whitespace());

        // then
        assert_eq!(cli.only_tasks_with_ids, vec!["one", "two"]);
    }

    #[test]
    fn tasks_can_be_listed() {
        // given
        let cli = "foo --list-tasks";

        // when
        let cli = Cli::parse_from(cli.split_whitespace());

        // then
        assert!(cli.list_tasks);
    }

    #[test]
    fn flavor_can_be_chosen() {
        // given
        let cli = "foo --flavor max";

        // when
        let cli = Cli::parse_from(cli.split_whitespace());

        // then
        assert_eq!(cli.flavor, Flavor::Max);
    }

    #[test]
    fn default_flavor_is_ci() {
        // given
        let cli = "foo";

        // when
        let cli = Cli::parse_from(cli.split_whitespace());

        // then
        assert_eq!(cli.flavor, Flavor::Ci);
    }
}
