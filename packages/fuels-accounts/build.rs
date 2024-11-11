fn main() {
    #[cfg(feature = "std")]
    {
        use std::fs;

        fs::create_dir_all("target").expect("Unable to create target directory");
        fs::write(
            "target/fuel-core-client-schema.sdl",
            fuel_core_client::SCHEMA_SDL,
        )
        .expect("Unable to write schema file");

        println!("cargo:rerun-if-changed=build.rs");
    }
}
