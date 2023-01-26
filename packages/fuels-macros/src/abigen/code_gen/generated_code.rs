use std::collections::HashSet;

use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::abigen::code_gen::type_path::TypePath;

#[derive(Default, Debug)]
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
        let mod_path = TypePath::new(mod_name).unwrap();
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
        let type_paths = self.types_with_unique_names();

        quote! {
            #(pub use #type_paths;)*
        }
    }

    fn types_with_unique_names(&self) -> Vec<&TypePath> {
        self.usable_types
            .iter()
            .sorted_by(|&lhs, &rhs| lhs.type_name().cmp(rhs.type_name()))
            .group_by(|&e| e.type_name())
            .into_iter()
            .filter_map(|(_, group)| {
                let mut types = group.collect::<Vec<_>>();
                (types.len() == 1).then_some(types.pop().unwrap())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::ident;

    #[test]
    fn will_wrap_code_in_mod() {
        let generated_code = GeneratedCode {
            code: quote! {some code},
            usable_types: HashSet::from([
                TypePath::new("SomeType").expect("Hand crafted, should be valid.")
            ]),
        };

        let generated_code = generated_code.wrap_in_mod(&ident("a_mod"));

        let expected_code = quote! {
            #[allow(clippy::too_many_arguments)]
            #[no_implicit_prelude]
            pub mod a_mod {
                some code
            }
        };

        assert_eq!(generated_code.code.to_string(), expected_code.to_string());
    }

    #[test]
    fn wrapping_in_mod_prepends_mod_to_usable_types() {
        let generated_code = GeneratedCode {
            code: quote! {some code},
            usable_types: HashSet::from([given_type_path("SomeType")]),
        };

        let generated_code = generated_code.wrap_in_mod(&ident("a_mod"));

        assert_eq!(
            generated_code.usable_types,
            HashSet::from([
                TypePath::new("a_mod::SomeType").expect("Hand crafted, should be valid!")
            ])
        );
    }

    #[test]
    fn appending_appends_both_code_and_usable_types() {
        // given
        let type_path_1 = given_type_path("SomeType1");
        let code_1 = GeneratedCode {
            code: quote! {some code 1},
            usable_types: HashSet::from([type_path_1.clone()]),
        };

        let type_path_2 = given_type_path("SomeType2");
        let code_2 = GeneratedCode {
            code: quote! {some code 2},
            usable_types: HashSet::from([type_path_2.clone()]),
        };

        // when
        let joined = code_1.append(code_2);

        // then
        assert_eq!(joined.code.to_string(), "some code 1 some code 2");
        assert_eq!(
            joined.usable_types,
            HashSet::from([type_path_1, type_path_2])
        )
    }

    #[test]
    fn use_statements_only_for_uniquely_named_types() {
        let generated_code = GeneratedCode {
            code: Default::default(),
            usable_types: HashSet::from([
                given_type_path("NotUnique"),
                given_type_path("some_mod::Unique"),
                given_type_path("even_though::the_duplicate_is::in_another_mod::NotUnique"),
            ]),
        };

        let use_statements = generated_code.use_statements_for_uniquely_named_types();

        assert_eq!(use_statements.to_string(), "pub use some_mod :: Unique ;");
    }

    fn given_type_path(path: &str) -> TypePath {
        TypePath::new(path).expect("Hand crafted, should be valid.")
    }
}
