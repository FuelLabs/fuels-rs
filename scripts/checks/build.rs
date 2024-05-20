use itertools::Itertools;
use std::path::Path;

fn main() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");

    let path = path
        .canonicalize()
        .unwrap_or_else(|_| panic!("Path not found: {path:?}"));

    let members = workspace_members(&path);

    generate_rust_code(&members);

    println!("cargo:rerun-if-changed={}", path.display());
}

fn workspace_members(cargo: &Path) -> Vec<String> {
    #[derive(Debug, Clone, serde::Deserialize)]
    struct Workspace {
        members: Vec<String>,
    }
    #[derive(Debug, Clone, serde::Deserialize)]
    struct Cargo {
        workspace: Workspace,
    }

    let data = std::fs::read_to_string(cargo).unwrap();
    let cargo_toml: Cargo = toml::from_str(&data).unwrap();
    cargo_toml.workspace.members
}

fn generate_rust_code(members: &[String]) {
    let members = members
        .iter()
        .map(|member| format!("{member:?}"))
        .join(",\n");

    let code = format!("static WORKSPACE_MEMBERS: &[&str] = &[{members}];");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("workspace_members.rs");
    std::fs::write(dest_path, code).unwrap();
}
