use std::{env, fs};

fn main() {
    //
    let std = env::var("CARGO_FEATURE_STD").is_ok();

    if std {
        fs::create_dir_all("target").expect("Unable to create target directory");
        fs::write(
            "target/fuel-core-client-schema.sdl",
            fuel_core_client::SCHEMA_SDL,
        )
        .expect("Unable to write schema file");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
