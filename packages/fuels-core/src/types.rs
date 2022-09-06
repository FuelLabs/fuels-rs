use anyhow::Result;
use fuels_types::errors::Error;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

use crate::{utils::ident, ParamType};
