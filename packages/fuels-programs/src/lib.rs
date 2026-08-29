#[cfg(feature = "std")]
pub mod calls;
#[cfg(feature = "std")]
pub mod contract;
#[cfg(feature = "std")]
pub mod executable;
#[cfg(feature = "std")]
pub mod responses;

pub const DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE: f32 = 0.50;

pub mod debug;

// `assembly` backs both `debug` (always compiled) and the `std`-only program APIs above.
// Without `std` the latter are gone, leaving parts of it unused.
#[cfg_attr(not(feature = "std"), allow(dead_code))]
pub(crate) mod assembly;
pub(crate) mod utils;
