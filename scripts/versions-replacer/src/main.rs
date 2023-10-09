use std::path::PathBuf;

use argh::FromArgs;
use color_eyre::{
    eyre::{eyre, Context},
    Result,
};
use regex::Regex;
use walkdir::WalkDir;

use versions_replacer::{
    metadata::collect_versions_from_cargo_toml, replace::replace_versions_in_file,
};

#[derive(FromArgs)]
/// Replace variables like '{{{{versions.fuels}}}}' with correct versions from Cargo.toml.
/// Uses versions from '[workspace.members]' and '[workspace.metadata.versions-replacer.external-versions]'.
struct VersionsReplacer {
    /// path to directory with files containing variables
    #[argh(positional)]
    path: PathBuf,
    /// path to Cargo.toml with versions
    #[argh(option)]
    manifest_path: PathBuf,
    /// regex to filter filenames (example: "\.md$")
    #[argh(option)]
    filename_regex: Option<Regex>,
}

fn main() -> Result<()> {
    let args: VersionsReplacer = argh::from_env();
    let versions = collect_versions_from_cargo_toml(&args.manifest_path)?;

    let mut total_replacements: Vec<usize> = Vec::new();

    for entry in WalkDir::new(&args.path) {
        let entry = entry.wrap_err("failed to get directory entry")?;

        if entry.path().is_file() {
            if let Some(filename_regex) = &args.filename_regex {
                let file_name = entry
                    .path()
                    .file_name()
                    .ok_or_else(|| eyre!("{:?} has an invalid file name", entry.path()))?
                    .to_str()
                    .ok_or_else(|| eyre!("filename is not valid UTF-8"))?;
                if !filename_regex.is_match(file_name) {
                    continue;
                }
            }

            let replacement_count = replace_versions_in_file(entry.path(), &versions)
                .wrap_err_with(|| format!("failed to replace versions in {:?}", entry.path()))?;
            if replacement_count > 0 {
                total_replacements.push(replacement_count);
            }
        }
    }

    println!(
        "replaced {} variables across {} files",
        total_replacements.iter().sum::<usize>(),
        total_replacements.len()
    );
    Ok(())
}
