use std::path::PathBuf;

use clap::{arg, Parser, ValueEnum};

#[derive(Parser)]
#[command(about = "Runs pre-release checks deemed to heavy for CI.")]
pub struct Cli {
    ///Comma separated list of crates to check.
    #[arg(
        short,
        long,
        value_delimiter = ',',
        num_args = 0..

    )]
    pub crates: Vec<String>,
    #[arg(short, long, default_value = "ci")]
    pub flavor: Flavor,

    /// Enable verbose output.
    #[arg(short, long, default_value = "false")]
    pub verbose: bool,

    #[arg(default_value = ".")]
    pub workspace_root: PathBuf,
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
    fn crates_can_be_listed() {
        // given
        let cli = "foo --crates one,two";

        // when
        let cli = Cli::parse_from(cli.split_whitespace());

        // then
        assert_eq!(cli.crates, vec!["one", "two"]);
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
