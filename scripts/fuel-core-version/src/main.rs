use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use color_eyre::{
    Result,
    eyre::{ContextCompat, OptionExt, bail},
};
use fuels_accounts::provider::SUPPORTED_FUEL_CORE_VERSION;
use semver::Version;
use toml::Value;

fn write_version_to_file(version: Version, version_file_path: impl AsRef<Path>) -> Result<()> {
    let Version {
        major,
        minor,
        patch,
        ..
    } = version;
    let text = format!("Version::new({major}, {minor}, {patch})");
    fs::write(version_file_path, text.as_bytes())?;
    Ok(())
}

fn get_version_file_path(
    manifest_path: impl AsRef<Path>,
) -> Result<PathBuf, color_eyre::eyre::Error> {
    Ok(manifest_path
        .as_ref()
        .parent()
        .wrap_err("Invalid manifest path")?
        .join("scripts/fuel-core-version/version.rs"))
}

fn verify_version_from_file(version: Version) -> Result<()> {
    if version != SUPPORTED_FUEL_CORE_VERSION {
        bail!(
            "fuel_core version in version.rs ({}) doesn't match one in Cargo.toml ({})",
            SUPPORTED_FUEL_CORE_VERSION,
            version
        );
    }
    println!(
        "fuel_core versions in versions.rs and Cargo.toml match ({})",
        version
    );
    Ok(())
}

#[derive(Debug, Parser)]
struct App {
    #[clap(subcommand)]
    command: Command,
    #[clap(long)]
    manifest_path: PathBuf,
}

#[derive(Debug, Subcommand)]
enum Command {
    Write,
    Verify,
}

fn main() -> Result<()> {
    let App {
        command,
        manifest_path,
    } = App::parse();
    let version = read_fuel_core_version(&manifest_path)?;
    let version_file_path = get_version_file_path(&manifest_path)?;
    match command {
        Command::Write => write_version_to_file(version, version_file_path)?,
        Command::Verify => verify_version_from_file(version)?,
    }
    Ok(())
}

pub fn read_fuel_core_version(path: impl AsRef<Path>) -> color_eyre::Result<Version> {
    let cargo_toml: Value = fs::read_to_string(path.as_ref())?.parse::<Value>()?;

    let str_version =
        find_dependency_version(&cargo_toml).ok_or_eyre("could not find fuel-core version")?;

    Ok(str_version.parse()?)
}

fn find_dependency_version(toml: &Value) -> Option<String> {
    match toml
        .get("workspace")?
        .get("dependencies")?
        .get("fuel-core")?
    {
        Value::String(version) => Some(version.clone()),
        Value::Table(table) => table
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from),
        _ => None,
    }
}
