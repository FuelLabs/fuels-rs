extern crate core;

use forc_pkg::{BuildOpts, Built, PkgOpts};
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn build(path: Option<String>) -> anyhow::Result<Built> {
    let pkg_opts = forc_pkg::PkgOpts {
        path,
        ..PkgOpts::default()
    };

    let build_opts = BuildOpts {
        pkg: pkg_opts,
        ..BuildOpts::default()
    };

    forc_pkg::build_with_options(build_opts)
}

pub async fn fmt(path: Option<String>) -> Result<(), anyhow::Error> {
    let this_dir = if let Some(ref path) = path {
        PathBuf::from(path)
    } else {
        std::env::current_dir()?
    };

    for e in WalkDir::new(this_dir.as_os_str())
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if e.metadata().unwrap().is_file() && e.file_name() == "Forc.toml" {
            let parent_dir = e.path().parent().unwrap().as_os_str().to_str().unwrap();
            tokio::process::Command::new("forc-fmt")
                .args(["--path", parent_dir, "--check"])
                .spawn()
                .expect("error: Couldn't read forc-fmt: No such file or directory. Please check if forc-fmt library is installed.");
        }
    }

    Ok(())
}
