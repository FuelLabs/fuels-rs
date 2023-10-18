use std::{borrow::Cow, collections::HashMap, fs, path::Path};

use color_eyre::{eyre::Context, Result};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};

pub static VERSIONS_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\{versions\.([\w_-]+)\}\}").unwrap());

pub fn replace_versions_in_file(
    path: impl AsRef<Path>,
    versions: &HashMap<String, String>,
) -> Result<usize> {
    let path = path.as_ref();
    let contents =
        fs::read_to_string(path).wrap_err_with(|| format!("failed to read {:?}", path))?;
    let (replaced_contents, replacement_count) = replace_versions_in_string(&contents, versions);
    if replacement_count > 0 {
        fs::write(path, replaced_contents.as_bytes())
            .wrap_err_with(|| format!("failed to write back to {:?}", path))?;
    }
    Ok(replacement_count)
}

pub fn replace_versions_in_string<'a>(
    s: &'a str,
    versions: &HashMap<String, String>,
) -> (Cow<'a, str>, usize) {
    let mut replacement_count = 0;
    let replaced_s = VERSIONS_REGEX.replace_all(s, |caps: &Captures| {
        if let Some(version) = versions.get(&caps[1]) {
            replacement_count += 1;
            version.clone()
        } else {
            // leave unchanged
            caps[0].to_string()
        }
    });
    (replaced_s, replacement_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_versions() -> HashMap<String, String> {
        [("fuels", "0.47.0"), ("fuel-types", "0.35.3")]
            .map(|(name, version)| (name.to_string(), version.to_string()))
            .into()
    }

    #[test]
    fn test_valid_replacements() {
        let s = "docs.rs/fuels/{{versions.fuels}}/fuels\ndocs.rs/fuel-types/{{versions.fuel-types}}/fuel-types";
        let versions = test_versions();
        let (replaced, count) = replace_versions_in_string(s, &versions);
        assert_eq!(
            replaced,
            format!(
                "docs.rs/fuels/{}/fuels\ndocs.rs/fuel-types/{}/fuel-types",
                versions["fuels"], versions["fuel-types"]
            )
        );
        assert_eq!(count, 2);
    }

    #[test]
    fn test_invalid_replacement() {
        let s = "```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deployed_contracts}}
```";
        let versions = test_versions();
        let (replaced, count) = replace_versions_in_string(s, &versions);
        assert_eq!(replaced, s);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_invalid_package_name() {
        let s = "docs.rs/fuels-wrong-name/{{versions.fuels-wrong-name}}/fuels-wrong-name";
        let versions = test_versions();
        let (replaced, count) = replace_versions_in_string(s, &versions);
        assert_eq!(replaced, s);
        assert_eq!(count, 0);
    }
}
