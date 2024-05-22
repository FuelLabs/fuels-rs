use itertools::Itertools;
use semver::Version;
use std::{collections::HashMap, path::Path, str::FromStr};

fn main() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");

    let path = path
        .canonicalize()
        .unwrap_or_else(|_| panic!("Path not found: {path:?}"));

    let cargo = workspace_cargo(&path);

    let fuel_core_version = extract_fuel_core_version(&cargo);

    generate_rust_code(&cargo.workspace.members, &fuel_core_version);

    println!("cargo:rerun-if-changed={}", path.display());
}

fn extract_fuel_core_version(cargo: &Cargo) -> Version {
    let fuel_core = cargo.workspace.dependencies.get("fuel-core").expect("fuel-core to be present in the workspace Cargo.toml so that we may use its version when doing compatibility checks in fuels-accounts");
    let version_str = fuel_core
        .version
        .clone()
        .expect("fuel-core dep in workspace Cargo.toml to have `version` field set");

    Version::from_str(&version_str).expect("fuel-core version to be a valid semver version")
}

fn workspace_cargo(cargo: &Path) -> Cargo {
    let data = std::fs::read_to_string(cargo).unwrap();
    toml::from_str(&data).unwrap()
}

fn generate_rust_code(members: &[String], fuel_core_version: &Version) {
    let members = members
        .iter()
        .map(|member| format!("{member:?}"))
        .join(",\n");

    let members_code =
        format!("#[allow(dead_code)] static WORKSPACE_MEMBERS: &[&str] = &[{members}];");
    let version_code = {
        let major = fuel_core_version.major;
        let minor = fuel_core_version.minor;
        let patch = fuel_core_version.patch;
        format!("#[allow(dead_code)] static FUEL_CORE_VERSION: ::semver::Version = ::semver::Version::new({major}, {minor}, {patch});")
    };
    let code = format!("{}\n{}", members_code, version_code);

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("workspace_cargo.rs");
    std::fs::write(dest_path, code).unwrap();
}

#[derive(Debug, Clone, serde::Deserialize)]
struct Dep {
    version: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct Workspace {
    members: Vec<String>,
    dependencies: HashMap<String, Dep>,
}
#[derive(Debug, Clone, serde::Deserialize)]
struct Cargo {
    workspace: Workspace,
}
