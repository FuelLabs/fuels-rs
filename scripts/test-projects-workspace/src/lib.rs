extern crate core;

// use forc_pkg::manifest::ManifestFile;
// use glob::glob;
// use std::path::PathBuf;
// use walkdir::WalkDir;
use forc_pkg::{BuildOpts, Built, PkgOpts};

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

// pub async fn fmt(path: Option<String>) -> Result<(), anyhow::Error> {
//
//     let this_dir = if let Some(ref path) = path {
//         PathBuf::from(path)
//     } else {
//         std::env::current_dir()?
//     };
//
//     let manifest_file = ManifestFile::from_dir(&this_dir)?.member_manifests()?;
//     let manifest_names = manifest_file.iter().map(|(a, _)| a).collect::<Vec<_>>();
//
//     for e in WalkDir::new(path.unwrap()).into_iter().filter_map(|e| e.ok()) {
//         if e.metadata().unwrap().is_dir() {
//             println!("{}", e.path().display());
//         }
//     }
//
//
// for entry in glob(together.as_str())? {
//     println!("{}", entry?.display());
// }
//
// Ok(())
//
// }
