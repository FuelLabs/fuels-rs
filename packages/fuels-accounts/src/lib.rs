#[cfg(feature = "std")]
mod account;
#[cfg(feature = "std")]
mod accounts_utils;
#[cfg(all(feature = "std", feature = "keystore"))]
pub mod keystore;
#[cfg(feature = "std")]
pub mod provider;
#[cfg(feature = "std")]
pub mod wallet;

#[cfg(feature = "std")]
pub use account::*;

#[cfg(feature = "coin-cache")]
mod coin_cache;

pub mod predicate;
pub mod signers;
#[cfg(test)]
mod test {
    #[test]
    fn sdl_is_the_same_as_from_fuel() {
        let file_sdl = include_str!("./schema/schema.sdl");

        let core_sdl = String::from_utf8(fuel_core_client::SCHEMA_SDL.to_vec()).unwrap();

        assert_eq!(file_sdl, &core_sdl);
    }
}
