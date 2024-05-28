use flate2::read::GzDecoder;
use semver::Version;
use std::{io::Cursor, path::Path};
use tar::Archive;

const CORE_VERSION: semver::Version = include!("../scripts/fuel-core-version/version.rs");
const EXECUTOR_FILE_NAME: &str = "fuel-core-wasm-executor.wasm";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let core_version_file =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/fuel-core-version/version.rs");
    println!("cargo:rerun-if-changed={}", core_version_file.display());

    let executor = expected_executor_location();
    if !executor.exists() {
        download_executor(&executor);
    }
}

fn expected_executor_location() -> std::path::PathBuf {
    let env = std::env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&env);

    std::fs::create_dir_all(out_dir).unwrap();

    out_dir.join(EXECUTOR_FILE_NAME)
}

fn download_executor(path: &Path) {
    const LINK_TEMPLATE: &str = "https://github.com/FuelLabs/fuel-core/releases/download/vVERSION/fuel-core-VERSION-x86_64-unknown-linux-gnu.tar.gz";
    let link = LINK_TEMPLATE.replace("VERSION", &CORE_VERSION.to_string());

    let response = reqwest::blocking::get(link).unwrap();
    assert!(
        response.status().is_success(),
        "Failed to download wasm executor"
    );

    let mut content = Cursor::new(response.bytes().unwrap());

    let mut archive = Archive::new(GzDecoder::new(&mut content));

    let mut extracted = false;
    let executor = Path::new(&format!(
        "fuel-core-{CORE_VERSION}-x86_64-unknown-linux-gnu"
    ))
    .join(EXECUTOR_FILE_NAME);

    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();

        if entry.path().unwrap() == executor {
            entry.unpack(path).unwrap();
            extracted = true;
            break;
        }
    }
    assert!(
        extracted,
        "Failed to extract wasm executor from the archive"
    );
}
