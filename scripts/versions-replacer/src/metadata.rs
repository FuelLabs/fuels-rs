use std::{collections::HashMap, path::Path};

use cargo_metadata::MetadataCommand;
use color_eyre::{Result, eyre::Context};
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
        .packages
        .iter()
        .map(|package| (package.name.clone(), package.version.to_string()))
        .collect::<HashMap<_, _>>();
    Ok(version_map)
}
