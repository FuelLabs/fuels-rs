use std::str::FromStr;

use proc_macro2::TokenStream;
use quote::quote;

use fuels_types::errors::Error;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct TypePath {
    path_parts: Vec<String>,
}

impl TypePath {
    pub fn new<T: ToString>(path: &T) -> Result<Self, Error> {
        let path_str = path.to_string();
        let path_parts = path_str
            .split("::")
            .map(|part| part.to_string())
            .collect::<Vec<_>>();

        if path_parts.is_empty() {
            Err(Error::InvalidType(format!(
                "TypePath cannot be constructed from {path_str} because it's empty!"
            )))
        } else {
            Ok(Self { path_parts })
        }
    }

    pub fn prepend(self, mut another: TypePath) -> Self {
        another.path_parts.extend(self.path_parts);
        another
    }

    pub fn type_name(&self) -> &str {
        self.path_parts
            .last()
            .expect("Must have at least one element")
            .as_str()
    }
}

impl From<&TypePath> for TokenStream {
    fn from(type_path: &TypePath) -> Self {
        let parts = type_path
            .path_parts
            .iter()
            .map(|part| TokenStream::from_str(part).unwrap());
        quote! {
            #(#parts)::*
        }
    }
}
impl From<TypePath> for TokenStream {
    fn from(type_path: TypePath) -> Self {
        (&type_path).into()
    }
}
