#[cfg(feature = "std")]
mod account;
#[cfg(feature = "std")]
mod accounts_utils;
#[cfg(feature = "std")]
pub mod impersonated_account;
#[cfg(feature = "std")]
pub mod provider;
#[cfg(feature = "std")]
pub mod wallet;

#[cfg(feature = "std")]
pub use account::*;

#[cfg(any(feature = "aws-kms-signer", feature = "google-kms-signer"))]
pub mod kms;

#[cfg(feature = "coin-cache")]
mod coin_cache;

pub mod predicate;
#[cfg(test)]
mod test {
    #[test]
    fn sdl_is_the_same_as_from_fuel() {
        let file_sdl = include_str!("./schema/schema.sdl");

        let core_sdl = String::from_utf8(fuel_core_client::SCHEMA_SDL.to_vec()).unwrap();

        assert_eq!(file_sdl, &core_sdl);
    }
}
