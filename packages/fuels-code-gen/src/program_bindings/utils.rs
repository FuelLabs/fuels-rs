use std::collections::{HashMap, HashSet};

use fuel_abi_types::abi::full_program::FullTypeApplication;
use inflector::Inflector;
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::Result,
    program_bindings::resolved_type::{GenericType, ResolvedType, TypeResolver},
    utils::{self, TypePath, safe_ident},
};

#[derive(Debug)]
pub(crate) struct Components {
    components: Vec<(Ident, ResolvedType)>,
}

impl Components {
    pub fn new(
        type_applications: &[FullTypeApplication],
        snake_case: bool,
        parent_module: TypePath,
    ) -> Result<Self> {
        let type_resolver = TypeResolver::new(parent_module);
        let components = type_applications
            .iter()
            .map(|type_application| {
                let name = if snake_case {
                    type_application.name.to_snake_case()
                } else {
                    type_application.name.to_owned()
                };

                let ident = safe_ident(&name);
                let ty = type_resolver.resolve(type_application)?;
                Result::Ok((ident, ty))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { components })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Ident, &ResolvedType)> {
        self.components.iter().map(|(ident, ty)| (ident, ty))
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    pub fn as_enum_variants(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.components.iter().map(|(ident, ty)| {
            if let ResolvedType::Unit = ty {
                quote! {#ident}
            } else {
                quote! {#ident(#ty)}
            }
        })
    }

    pub fn generate_parameters_for_unused_generics(
        &self,
        declared_generics: &[Ident],
    ) -> (Vec<Ident>, Vec<TokenStream>) {
        self.unused_named_generics(declared_generics)
            .enumerate()
            .map(|(index, generic)| {
                let ident = utils::ident(&format!("_unused_generic_{index}"));
                let ty = quote! {::core::marker::PhantomData<#generic>};
                (ident, ty)
            })
            .unzip()
    }

    pub fn generate_variant_for_unused_generics(
        &self,
        declared_generics: &[Ident],
    ) -> Option<TokenStream> {
        let phantom_types = self
            .unused_named_generics(declared_generics)
            .map(|generic| {
                quote! {::core::marker::PhantomData<#generic>}
            })
            .collect_vec();

        (!phantom_types.is_empty()).then(|| {
            quote! {
                #[Ignore]
                IgnoreMe(#(#phantom_types),*)
            }
        })
    }

    fn named_generics(&self) -> HashSet<Ident> {
        self.components
            .iter()
            .flat_map(|(_, ty)| ty.generics())
            .filter_map(|generic_type| {
                if let GenericType::Named(name) = generic_type {
                    Some(name)
                } else {
                    None
                }
            })
            .collect()
    }

    fn unused_named_generics<'a>(
        &'a self,
        declared_generics: &'a [Ident],
    ) -> impl Iterator<Item = &'a Ident> {
        let used_generics = self.named_generics();
        declared_generics
            .iter()
            .filter(move |generic| !used_generics.contains(generic))
    }
}

pub(crate) fn tokenize_generics(generics: &[Ident]) -> (TokenStream, TokenStream) {
    if generics.is_empty() {
        return (Default::default(), Default::default());
    }

    (
        quote! {<#(#generics,)*>},
        quote! {<#(#generics: ::fuels::core::traits::Tokenizable + ::fuels::core::traits::Parameterize, )*>},
    )
}

pub(crate) fn sdk_provided_custom_types_lookup() -> HashMap<TypePath, TypePath> {
    [
        ("std::address::Address", "::fuels::types::Address"),
        ("std::asset_id::AssetId", "::fuels::types::AssetId"),
        ("std::b512::B512", "::fuels::types::B512"),
        ("std::bytes::Bytes", "::fuels::types::Bytes"),
        ("std::contract_id::ContractId", "::fuels::types::ContractId"),
        ("std::identity::Identity", "::fuels::types::Identity"),
        ("std::option::Option", "::core::option::Option"),
        ("std::result::Result", "::core::result::Result"),
        ("std::string::String", "::std::string::String"),
        ("std::vec::Vec", "::std::vec::Vec"),
        (
            "std::vm::evm::evm_address::EvmAddress",
            "::fuels::types::EvmAddress",
        ),
    ]
    .into_iter()
    .map(|(original_type_path, provided_type_path)| {
        let msg = "known at compile time to be correctly formed";
        (
            TypePath::new(original_type_path).expect(msg),
            TypePath::new(provided_type_path).expect(msg),
        )
    })
    .collect()
}

pub(crate) fn get_equivalent_bech32_type(ttype: &ResolvedType) -> Option<TokenStream> {
    let ResolvedType::StructOrEnum { path, .. } = ttype else {
        return None;
    };

    match path.to_string().as_str() {
        "::fuels::types::Address" => Some(quote! {::fuels::types::bech32::Bech32Address}),
        "::fuels::types::ContractId" => Some(quote! {::fuels::types::bech32::Bech32ContractId}),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use fuel_abi_types::abi::full_program::FullTypeDeclaration;

    use super::*;

    #[test]
    fn respects_snake_case_flag() -> Result<()> {
        // given
        let type_application = type_application_named("WasNotSnakeCased");

        // when
        let sut = Components::new(&[type_application], true, TypePath::default())?;

        // then
        assert_eq!(sut.iter().next().unwrap().0, "was_not_snake_cased");

        Ok(())
    }

    #[test]
    fn avoids_collisions_with_reserved_keywords() -> Result<()> {
        {
            let type_application = type_application_named("if");

            let sut = Components::new(&[type_application], false, TypePath::default())?;

            assert_eq!(sut.iter().next().unwrap().0, "if_");
        }

        {
            let type_application = type_application_named("let");

            let sut = Components::new(&[type_application], false, TypePath::default())?;

            assert_eq!(sut.iter().next().unwrap().0, "let_");
        }

        Ok(())
    }

    fn type_application_named(name: &str) -> FullTypeApplication {
        FullTypeApplication {
            name: name.to_string(),
            type_decl: FullTypeDeclaration {
                type_field: "u64".to_string(),
                components: vec![],
                type_parameters: vec![],
            },
            type_arguments: vec![],
        }
    }
}
