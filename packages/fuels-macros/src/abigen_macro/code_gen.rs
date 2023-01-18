//! This module implements everything related to code generation/expansion
//! from a FuelVM ABI.
mod abi_types;
mod abigen;
mod custom_types;
mod generated_code;
mod resolved_type;
mod source;
mod type_path;
mod utils;

pub(crate) use abigen::{Abigen, AbigenTarget, ProgramType};
