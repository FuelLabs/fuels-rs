//! This module implements everything related to code generation/expansion
//! from a fuel/sway ABI.
pub mod abigen;
pub mod bindings;
pub mod custom_types;
pub mod docs_gen;
pub mod function_selector;
pub mod functions_gen;
mod resolved_type;

pub use abigen::{extract_and_parse_logs, create_log_data_param_type_pairs};

