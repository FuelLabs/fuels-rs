use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/workspace_cargo.rs"));

pub fn verify_core_version(fuels_accounts: &Path) -> anyhow::Result<()> {
    let contents = std::fs::read_to_string(fuels_accounts.join("./src/provider/version.rs"))?;

    let correct_version = &self::FUEL_CORE_VERSION;
    let apply_template = |version: &semver::Version| -> String {
        let major = version.major;
        let minor = version.minor;
        let patch = version.patch;
        format!("pub(crate) const SUPPORTED_FUEL_CORE_VERSION: ::semver::Version = ::semver::Version::new({major}, {minor}, {patch});\n")
    };

    let expected_contents = apply_template(correct_version);
    let diff = pretty_assertions::StrComparison::new(&expected_contents, &contents);
    if contents != expected_contents {
        return Err(anyhow::anyhow!("Fuel core version mismatch. {diff}"));
    }

    Ok(())
}
