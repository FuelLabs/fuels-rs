use semver::Version;

pub fn get_supported_fuel_core_version() -> Version {
    "0.20.6".parse().unwrap()
}

pub struct VersionCompatibility {
    pub supported_version: Version,
    pub is_major_supported: bool,
    pub is_minor_supported: bool,
    pub is_patch_supported: bool,
}

pub fn check_fuel_core_version_compatibility(
    network_version: &str,
) -> Result<VersionCompatibility, semver::Error> {
    let network_version = network_version.parse::<Version>()?;
    let supported_version = get_supported_fuel_core_version();

    let is_major_supported = supported_version.major == network_version.major;
    let is_minor_supported = supported_version.minor == network_version.minor;
    let is_patch_supported = supported_version.patch == network_version.patch;

    Ok(VersionCompatibility {
        supported_version,
        is_major_supported,
        is_minor_supported,
        is_patch_supported,
    })
}
