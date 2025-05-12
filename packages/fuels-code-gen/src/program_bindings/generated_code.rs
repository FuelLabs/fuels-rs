use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::utils::TypePath;

#[derive(Default, Debug)]
pub(crate) struct GeneratedCode {
    top_level_code: TokenStream,
    usable_types: HashSet<TypePath>,
    code_in_mods: HashMap<Ident, GeneratedCode>,
    no_std: bool,
}

impl GeneratedCode {
    pub fn new(code: TokenStream, usable_types: HashSet<TypePath>, no_std: bool) -> Self {
        Self {
            top_level_code: code,
            code_in_mods: HashMap::default(),
            usable_types,
            no_std,
        }
    }

    fn prelude(&self) -> TokenStream {
        let lib = if self.no_std {
            quote! {::alloc}
        } else {
            quote! {::std}
        };

        quote! {
                use ::core::{
                    clone::Clone,
                    convert::{Into, TryFrom, From},
                    iter::IntoIterator,
                    iter::Iterator,
                    marker::Sized,
                    panic,
                };

                use #lib::{string::ToString, format, vec, default::Default};

        }
    }

    pub fn code(&self) -> TokenStream {
        let top_level_code = &self.top_level_code;

        let prelude = self.prelude();
        let code_in_mods = self
            .code_in_mods
            .iter()
            .sorted_by_key(|(mod_name, _)| {
                // Sorted to make test expectations maintainable
                *mod_name
            })
            .map(|(mod_name, generated_code)| {
                let code = generated_code.code();
                quote! {
                    #[allow(clippy::too_many_arguments)]
                    #[allow(clippy::disallowed_names)]
                    #[no_implicit_prelude]
                    pub mod #mod_name {
                        #prelude
                        #code
                    }
                }
            });

        quote! {
            #top_level_code
            #(#code_in_mods)*
        }
    }

    pub fn is_empty(&self) -> bool {
        self.code().is_empty()
    }

    pub fn merge(mut self, another: GeneratedCode) -> Self {
        self.top_level_code.extend(another.top_level_code);
        self.usable_types.extend(another.usable_types);

        for (mod_name, code) in another.code_in_mods {
            let entry = self.code_in_mods.entry(mod_name).or_default();
            *entry = std::mem::take(entry).merge(code);
        }

        self
    }

    pub fn wrap_in_mod(mut self, mod_name: impl Into<TypePath>) -> Self {
        let mut parts = mod_name.into().take_parts();
        parts.reverse();

        for mod_name in parts {
            self = self.wrap_in_single_mod(mod_name)
        }

        self
    }

    fn wrap_in_single_mod(self, mod_name: Ident) -> Self {
        Self {
            code_in_mods: HashMap::from([(mod_name, self)]),
            ..Default::default()
        }
    }

    pub fn use_statements_for_uniquely_named_types(&self) -> TokenStream {
        let type_paths = self
            .types_with_unique_names()
            .into_iter()
            .filter(|type_path| type_path.has_multiple_parts());

        quote! {
            #(pub use #type_paths;)*
        }
    }

    fn types_with_unique_names(&self) -> Vec<TypePath> {
        self.code_in_mods
            .iter()
            .flat_map(|(mod_name, code)| {
                code.types_with_unique_names()
                    .into_iter()
                    .map(|type_path| type_path.prepend(mod_name.into()))
                    .collect::<Vec<_>>()
            })
            .chain(self.usable_types.iter().cloned())
            .sorted_by(|lhs, rhs| lhs.ident().cmp(&rhs.ident()))
            .group_by(|e| e.ident().cloned())
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
    fn can_merge_top_level_code() {
        // given
        let struct_1 = given_some_struct_code("Struct1");
        let struct_2 = given_some_struct_code("Struct2");

        // when
        let joined = struct_1.merge(struct_2);

        // then
        let expected_code = quote! {
            struct Struct1;
            struct Struct2;
        };

        assert_eq!(joined.code().to_string(), expected_code.to_string());
    }

    #[test]
    fn wrapping_in_mod_updates_code() {
        // given
        let some_type = given_some_struct_code("SomeType");

        // when
        let wrapped_in_mod = some_type.wrap_in_mod(given_type_path("a_mod"));

        // then
        let expected_code = quote! {
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::disallowed_names)]
            #[no_implicit_prelude]
            pub mod a_mod {
                use ::core::{
                    clone::Clone,
                    convert::{Into, TryFrom, From},
                    iter::IntoIterator,
                    iter::Iterator,
                    marker::Sized,
                    panic,
                };

                use ::std::{string::ToString, format, vec, default::Default};

                struct SomeType;
            }
        };

        assert_eq!(wrapped_in_mod.code().to_string(), expected_code.to_string());
    }

    #[test]
    fn wrapping_in_mod_updates_use_statements() {
        // given
        let some_type = given_some_struct_code("SomeType");
        let wrapped_in_mod = some_type.wrap_in_mod(given_type_path("a_mod"));

        // when
        let use_statements = wrapped_in_mod.use_statements_for_uniquely_named_types();

        // then
        let expected_use_statements = quote! {pub use a_mod::SomeType;};
        assert_eq!(
            use_statements.to_string(),
            expected_use_statements.to_string()
        );
    }

    #[test]
    fn merging_code_will_merge_mods_as_well() {
        // given
        let common_struct_1 = given_some_struct_code("SomeStruct1")
            .wrap_in_mod(given_type_path("common_mod::deeper_mod"));

        let common_struct_2 =
            given_some_struct_code("SomeStruct2").wrap_in_mod(given_type_path("common_mod"));

        let top_level_struct = given_some_struct_code("TopLevelStruct");

        let different_mod_struct =
            given_some_struct_code("SomeStruct3").wrap_in_mod(given_type_path("different_mod"));

        // when
        let merged_code = common_struct_1
            .merge(common_struct_2)
            .merge(top_level_struct)
            .merge(different_mod_struct);

        // then
        let prelude = quote! {
                use ::core::{
                    clone::Clone,
                    convert::{Into, TryFrom, From},
                    iter::IntoIterator,
                    iter::Iterator,
                    marker::Sized,
                    panic,
                };
                use ::std::{string::ToString, format, vec, default::Default};
        };

        let expected_code = quote! {
            struct TopLevelStruct;
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::disallowed_names)]
            #[no_implicit_prelude]
            pub mod common_mod {
                #prelude

                struct SomeStruct2;
                #[allow(clippy::too_many_arguments)]
                #[allow(clippy::disallowed_names)]
                #[no_implicit_prelude]
                pub mod deeper_mod {
                    #prelude
                    struct SomeStruct1;
                }
            }
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::disallowed_names)]
            #[no_implicit_prelude]
            pub mod different_mod {
                #prelude
                struct SomeStruct3;
            }
        };

        let code = merged_code.code();
        assert_eq!(code.to_string(), expected_code.to_string());

        let use_statements = merged_code.use_statements_for_uniquely_named_types();
        let expected_use_statements = quote! {
            pub use common_mod::deeper_mod::SomeStruct1;
            pub use common_mod::SomeStruct2;
            pub use different_mod::SomeStruct3;
        };
        assert_eq!(
            use_statements.to_string(),
            expected_use_statements.to_string()
        );
    }

    #[test]
    fn use_statement_not_generated_for_top_level_type() {
        let usable_types = ["TopLevelImport", "something::Deeper"]
            .map(given_type_path)
            .into_iter()
            .collect();
        let code = GeneratedCode::new(Default::default(), usable_types, false);

        let use_statements = code.use_statements_for_uniquely_named_types();

        let expected_use_statements = quote! {
            pub use something::Deeper;
        };
        assert_eq!(
            use_statements.to_string(),
            expected_use_statements.to_string()
        );
    }

    #[test]
    fn use_statements_only_for_uniquely_named_types() {
        // given
        let not_unique_struct =
            given_some_struct_code("NotUnique").wrap_in_mod(TypePath::new("another_mod").unwrap());

        let generated_code = GeneratedCode::new(
            Default::default(),
            HashSet::from([
                given_type_path("some_mod::Unique"),
                given_type_path("even_though::the_duplicate_is::in_another_mod::NotUnique"),
            ]),
            false,
        )
        .merge(not_unique_struct);

        // when
        let use_statements = generated_code.use_statements_for_uniquely_named_types();

        // then
        let expected_use_statements = quote! {
            pub use some_mod::Unique;
        };

        assert_eq!(
            use_statements.to_string(),
            expected_use_statements.to_string()
        );
    }

    fn given_some_struct_code(struct_name: &str) -> GeneratedCode {
        let struct_ident = ident(struct_name);

        GeneratedCode::new(
            quote! {struct #struct_ident;},
            HashSet::from([given_type_path(struct_name)]),
            false,
        )
    }

    fn given_type_path(path: &str) -> TypePath {
        TypePath::new(path).expect("hand crafted, should be valid")
    }
}
