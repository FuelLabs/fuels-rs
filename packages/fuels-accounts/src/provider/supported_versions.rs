use crate::provider::supported_fuel_core_version::SUPPORTED_FUEL_CORE_VERSION;
use semver::Version;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct VersionCompatibility {
    pub(crate) supported_version: Version,
    pub(crate) is_major_supported: bool,
    pub(crate) is_minor_supported: bool,
    pub(crate) is_patch_supported: bool,
}

pub(crate) fn compare_node_compatibility(network_version: Version) -> VersionCompatibility {
    check_version_compatibility(network_version, SUPPORTED_FUEL_CORE_VERSION)
}

fn check_version_compatibility(
    actual_version: Version,
    expected_version: Version,
) -> VersionCompatibility {
    let is_major_supported = expected_version.major == actual_version.major;
    let is_minor_supported = expected_version.minor == actual_version.minor;
    let is_patch_supported = expected_version.patch == actual_version.patch;

    VersionCompatibility {
        supported_version: expected_version,
        is_major_supported,
        is_minor_supported,
        is_patch_supported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_validate_all_possible_version_mismatches() {
        let expected_version = "0.1.2".parse::<Version>().unwrap();

        assert_eq!(
            check_version_compatibility("1.1.2".parse().unwrap(), expected_version.clone()),
            VersionCompatibility {
                is_major_supported: false,
                is_minor_supported: true,
                is_patch_supported: true,
                supported_version: expected_version.clone()
            }
        );

        assert_eq!(
            check_version_compatibility("1.2.2".parse().unwrap(), expected_version.clone()),
            VersionCompatibility {
                is_major_supported: false,
                is_minor_supported: false,
                is_patch_supported: true,
                supported_version: expected_version.clone()
            }
        );

        assert_eq!(
            check_version_compatibility("1.1.3".parse().unwrap(), expected_version.clone()),
            VersionCompatibility {
                is_major_supported: false,
                is_minor_supported: true,
                is_patch_supported: false,
                supported_version: expected_version.clone()
            }
        );

        assert_eq!(
            check_version_compatibility("0.2.2".parse().unwrap(), expected_version.clone()),
            VersionCompatibility {
                is_major_supported: true,
                is_minor_supported: false,
                is_patch_supported: true,
                supported_version: expected_version.clone()
            }
        );

        assert_eq!(
            check_version_compatibility("0.2.3".parse().unwrap(), expected_version.clone()),
            VersionCompatibility {
                is_major_supported: true,
                is_minor_supported: false,
                is_patch_supported: false,
                supported_version: expected_version.clone()
            }
        );

        assert_eq!(
            check_version_compatibility("0.1.3".parse().unwrap(), expected_version.clone()),
            VersionCompatibility {
                is_major_supported: true,
                is_minor_supported: true,
                is_patch_supported: false,
                supported_version: expected_version.clone()
            }
        );

        assert_eq!(
            check_version_compatibility("0.1.2".parse().unwrap(), expected_version.clone()),
            VersionCompatibility {
                is_major_supported: true,
                is_minor_supported: true,
                is_patch_supported: true,
                supported_version: expected_version.clone()
            }
        );
    }
}
