#[cfg(feature = "std")]
pub mod calls;
#[cfg(feature = "std")]
pub mod contract;
#[cfg(feature = "std")]
pub mod executable;
#[cfg(feature = "std")]
pub mod responses;

pub const DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE: f32 = 0.10;

pub mod debug;

pub(crate) mod assembly;
pub(crate) mod utils;
