use anyhow::{anyhow, bail};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Output;

use crate::{cli::RunConfig, types::ResultWriter};

use crate::types::Manifest;
use walkdir::WalkDir;

pub mod cli;
pub mod types;

pub async fn run_command(config: &RunConfig) -> Output {
    tokio::process::Command::new(&config.prepared_command.command)
        .args(&config.prepared_command.args)
        .arg(&config.project_path)
        .output()
        .await
        .expect("failed to run command")
}

pub fn display_info(result_writer: &mut ResultWriter, config: &RunConfig, output_result: Output) {
    result_writer
        .display_info(config, output_result)
        .expect("could not display build info");
}

pub fn check_workspace(path: &Path, result_writer: &mut ResultWriter) -> anyhow::Result<()> {
    let workspace_forc_toml_path = path.join("Forc.toml").canonicalize()?;

    let workspace_root_path = workspace_forc_toml_path
        .parent()
        .ok_or_else(|| anyhow!("Cannot get parent dir of {:?}", &workspace_forc_toml_path))?;

    let workspace_forc_toml_str =
        std::fs::read_to_string(&workspace_forc_toml_path).map_err(|e| {
            anyhow!(
                "failed to read manifest at {:?}: {}",
                &workspace_forc_toml_path,
                e
            )
        })?;
    let toml_de = &mut toml::de::Deserializer::new(&workspace_forc_toml_str);
    let mut parsed_workspace_forc_toml: Manifest = serde::Deserialize::deserialize(toml_de)?;

    validate(&parsed_workspace_forc_toml, workspace_root_path)?;

    let tests_workspace_members_paths = parsed_workspace_forc_toml
        .workspace
        .members
        .iter()
        .map(|path| {
            workspace_root_path.join(path).canonicalize().expect(
                "Could not canonicalize member path in parsed_toml.workspace.members struct",
            )
        })
        .collect::<Vec<_>>();

    let existing_tests_paths = find_all_forc_tomls_on_root(workspace_root_path);
    let missing_tests_paths = compare_vec(&tests_workspace_members_paths, &existing_tests_paths);

    if !missing_tests_paths.is_empty() {
        result_writer.display_warning(&format!(
            "\n{:>83}",
            "There are a few tests missing from Force.toml that will automatically be included"
        ))?;

        let dec_missing_members = missing_tests_paths
            .iter()
            .map(|path| remove_before_parent(path, &workspace_root_path.to_path_buf()))
            .collect::<Vec<PathBuf>>();

        add_members_to_workspace(
            dec_missing_members,
            &mut parsed_workspace_forc_toml,
            &workspace_forc_toml_path,
        );
        check_workspace(workspace_root_path, result_writer).expect("Failed to check workspace");
    }
    Ok(())
}

pub fn validate(toml: &Manifest, path: &Path) -> anyhow::Result<()> {
    for member in toml.workspace.members.iter() {
        let member_path = path.join(member).join("Forc.toml");
        if !member_path.exists() {
            bail!(
                "{:?} is listed as a member of the workspace but {:?} does not exists",
                &member,
                member_path
            );
        }
    }
    Ok(())
}

fn find_all_forc_tomls_on_root(root: &Path) -> Vec<PathBuf> {
    let mut paths = vec![];
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.file_name().expect("Failed to get file name") == "Forc.toml" {
            let parent = path.parent().expect("Failed to get parent");
            if parent != Path::new(root) {
                paths.push(parent.to_path_buf());
            }
        }
    }
    paths
}

fn compare_vec(a: &[PathBuf], b: &[PathBuf]) -> Vec<PathBuf> {
    let missing_paths: Vec<PathBuf> = b.iter().filter(|path| !a.contains(path)).cloned().collect();
    missing_paths
}

fn remove_before_parent(path: &Path, parent: &PathBuf) -> PathBuf {
    let canon_path = path.canonicalize().expect("Could not canonicalize path");
    let parent_path = PathBuf::from(parent);
    let new_path = canon_path
        .strip_prefix(parent_path)
        .expect("Could not strip prefix");
    new_path.to_path_buf()
}

fn add_members_to_workspace(
    missing_members: Vec<PathBuf>,
    workspace: &mut Manifest,
    workspace_forc_toml_path: &PathBuf,
) {
    let members = missing_members;
    workspace.workspace.members.extend(members);
    workspace.workspace.members.sort();
    let new_toml = toml::to_string_pretty(&workspace).expect("Failed to string_pretty new toml");
    let mut file = File::create(workspace_forc_toml_path).expect("Failed to create file");
    file.write_all(new_toml.as_bytes())
        .expect("Failed to write new Forc.toml");
}
