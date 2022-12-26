use std::collections::HashSet;

use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::code_gen::type_path::TypePath;

#[derive(Default)]
pub(crate) struct GeneratedCode {
    pub code: TokenStream,
    pub usable_types: HashSet<TypePath>,
}

impl GeneratedCode {
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    pub fn append(mut self, another: GeneratedCode) -> Self {
        self.code.extend(another.code);
        self.usable_types.extend(another.usable_types);
        self
    }

    pub fn wrap_in_mod(self, mod_name: &Ident) -> Self {
        let mod_path = TypePath::new(&mod_name).unwrap();
        let type_paths = self
            .usable_types
            .into_iter()
            .map(|type_path| type_path.prepend(mod_path.clone()))
            .collect();

        let inner_code = self.code;
        let code = quote! {
            #[allow(clippy::too_many_arguments)]
            #[no_implicit_prelude]
            pub mod #mod_name {
                #inner_code
            }
        };

        Self {
            code,
            usable_types: type_paths,
        }
    }

    pub fn use_statements_for_uniquely_named_types(&self) -> TokenStream {
        let type_paths = self
            .types_with_unique_type_name()
            .into_iter()
            .map(TokenStream::from);

        quote! {
            #(pub use #type_paths;)*
        }
    }

    fn types_with_unique_type_name(&self) -> Vec<&TypePath> {
        self.usable_types
            .iter()
            .sorted_by(|&lhs, &rhs| lhs.type_name().cmp(rhs.type_name()))
            .group_by(|&e| e.type_name())
            .into_iter()
            .filter_map(|(_, group)| {
                let mut types = group.collect::<Vec<_>>();
                if types.len() == 1 {
                    Some(types.pop().unwrap())
                } else {
                    None
                }
            })
            .collect()
    }
}
