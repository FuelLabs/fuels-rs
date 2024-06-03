use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use color_eyre::{
    eyre::{bail, ContextCompat},
    Result,
};
use fuels_accounts::provider::SUPPORTED_FUEL_CORE_VERSION;
use semver::Version;
use versions_replacer::metadata::collect_versions_from_cargo_toml;

fn get_version_from_toml(manifest_path: impl AsRef<Path>) -> Result<Version> {
    let versions = collect_versions_from_cargo_toml(manifest_path)?;
    let version = versions["fuel-core-types"].parse::<Version>()?;
    Ok(version)
}

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
    let version = get_version_from_toml(&manifest_path)?;
    let version_file_path = get_version_file_path(&manifest_path)?;
    match command {
        Command::Write => write_version_to_file(version, version_file_path)?,
        Command::Verify => verify_version_from_file(version)?,
    }
    Ok(())
}
