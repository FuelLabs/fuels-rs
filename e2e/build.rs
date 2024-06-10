use flate2::read::GzDecoder;
use fuels_accounts::provider::SUPPORTED_FUEL_CORE_VERSION;
use std::{
    io::Cursor,
    path::{Path, PathBuf},
};
use tar::Archive;

struct Downloader {
    dir: PathBuf,
}

impl Downloader {
    const EXECUTOR_FILE_NAME: &'static str = "fuel-core-wasm-executor.wasm";

    pub fn new() -> Self {
        let env = std::env::var("OUT_DIR").unwrap();
        let out_dir = Path::new(&env);
        Self {
            dir: out_dir.to_path_buf(),
        }
    }

    pub fn should_download(&self) -> anyhow::Result<bool> {
        if !self.executor_path().exists() {
            return Ok(true);
        }

        if !self.version_path().exists() {
            return Ok(true);
        }

        let saved_version = semver::Version::parse(&std::fs::read_to_string(self.version_path())?)?;
        if saved_version != SUPPORTED_FUEL_CORE_VERSION {
            return Ok(true);
        }

        Ok(false)
    }

    pub fn download(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.dir)?;

        const LINK_TEMPLATE: &str = "https://github.com/FuelLabs/fuel-core/releases/download/vVERSION/fuel-core-VERSION-x86_64-unknown-linux-gnu.tar.gz";
        let link = LINK_TEMPLATE.replace("VERSION", &SUPPORTED_FUEL_CORE_VERSION.to_string());

        let response = reqwest::blocking::get(link)?;
        if !response.status().is_success() {
            anyhow::bail!("Failed to download wasm executor: {}", response.status());
        }

        let mut content = Cursor::new(response.bytes()?);

        let mut archive = Archive::new(GzDecoder::new(&mut content));

        let mut extracted = false;
        let executor_in_tar = Path::new(&format!(
            "fuel-core-{SUPPORTED_FUEL_CORE_VERSION}-x86_64-unknown-linux-gnu"
        ))
        .join(Self::EXECUTOR_FILE_NAME);

        for entry in archive.entries()? {
            let mut entry = entry?;

            if entry.path()? == executor_in_tar {
                entry.unpack(self.executor_path())?;
                std::fs::write(
                    self.version_path(),
                    format!("{SUPPORTED_FUEL_CORE_VERSION}"),
                )?;

                extracted = true;
                break;
            }
        }
        if !extracted {
            anyhow::bail!("Failed to extract wasm executor from the archive");
        }

        Ok(())
    }

    fn make_cargo_watch_downloaded_files(&self) {
        let executor_path = self.executor_path();
        println!("cargo:rerun-if-changed={}", executor_path.display());

        let version_path = self.version_path();
        println!("cargo:rerun-if-changed={}", version_path.display());
    }

    fn executor_path(&self) -> PathBuf {
        self.dir.join(Self::EXECUTOR_FILE_NAME)
    }

    fn version_path(&self) -> PathBuf {
        self.dir.join("version.rs")
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let downloader = Downloader::new();
    downloader.make_cargo_watch_downloaded_files();

    if downloader.should_download().unwrap() {
        downloader.download().unwrap();
    }
}
