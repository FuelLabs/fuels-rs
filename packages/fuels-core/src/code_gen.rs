//! This module implements everything related to code generation/expansion
//! from a FuelVM ABI.
pub mod abi_types;
pub mod abigen;
pub mod custom_types;
pub mod function_selector;
mod generated_code;
mod resolved_type;
mod type_path;
mod utils;
