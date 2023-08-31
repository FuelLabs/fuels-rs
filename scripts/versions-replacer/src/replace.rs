use std::{collections::HashMap, fs, path::Path};

use color_eyre::{eyre::Context, Result};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};

pub static VERSIONS_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\{versions\.([\w_-]+)\}\}").unwrap());

pub fn replace_versions_in_file(
    path: impl AsRef<Path>,
    versions: &HashMap<String, String>,
) -> Result<()> {
    let path = path.as_ref();
    let contents =
        fs::read_to_string(path).wrap_err_with(|| format!("failed to read {:?}", path))?;
    let replaced_contents =
        VERSIONS_REGEX.replace_all(&contents, |caps: &Captures| &versions[&caps[1]]);
    fs::write(path, replaced_contents.as_bytes())
        .wrap_err_with(|| format!("failed to write back to {:?}", path))?;
    Ok(())
}
