use std::{collections::HashMap, path::Path};

use cargo_metadata::MetadataCommand;
use color_eyre::{eyre::Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WorkspaceMetadata {
    pub versions_replacer: VersionsReplacerMetadata,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VersionsReplacerMetadata {
    pub external_versions: HashMap<String, String>,
}

pub fn collect_versions_from_cargo_toml(
    manifest_path: impl AsRef<Path>,
) -> Result<HashMap<String, String>> {
    let metadata = MetadataCommand::new()
        .manifest_path(manifest_path.as_ref())
        .exec()
        .wrap_err("failed to execute 'cargo metadata'")?;
    let version_map = metadata
        .workspace_members
        .iter()
        .map(|package_id| {
            let package = &metadata[package_id];
            (package.name.clone(), package.version.to_string())
        })
        .chain(
            serde_json::from_value::<WorkspaceMetadata>(metadata.workspace_metadata.clone())
                .wrap_err("failed to parse '[workspace.metadata]'")?
                .versions_replacer
                .external_versions
        )
        .collect::<HashMap<_, _>>();
    Ok(version_map)
}
