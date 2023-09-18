use std::collections::HashSet;

use fuel_abi_types::abi::full_program::FullTypeDeclaration;
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::{error, Result},
    program_bindings::{
        custom_types::utils::{extract_components, extract_generic_parameters},
        generated_code::GeneratedCode,
        resolved_type::{GenericType, ResolvedType},
        utils::Component,
    },
};

/// Returns a TokenStream containing the declaration, `Parameterize`,
/// `Tokenizable` and `TryFrom` implementations for the enum described by the
/// given TypeDeclaration.
pub(crate) fn expand_custom_enum(
    type_decl: &FullTypeDeclaration,
    no_std: bool,
) -> Result<GeneratedCode> {
    let enum_type_path = type_decl.custom_type_path()?;
    let enum_ident = enum_type_path.ident().unwrap();

    let components = extract_components(type_decl, false, &enum_type_path.parent())?;
    if components.is_empty() {
        return Err(error!("Enum must have at least one component!"));
    }
    let generics = extract_generic_parameters(type_decl);
    // TODO: segfault impl Default when all elements are PhantomDatas

    let code = enum_decl(enum_ident, &components, &generics, no_std);

    let enum_code = GeneratedCode::new(code, HashSet::from([enum_ident.into()]), no_std);

    Ok(enum_code.wrap_in_mod(enum_type_path.parent()))
}

fn enum_decl(
    enum_ident: &Ident,
    components: &[Component],
    generics: &[Ident],
    no_std: bool,
) -> TokenStream {
    let enum_variants = components.iter().map(
        |Component {
             field_name,
             field_type,
         }| {
            if let ResolvedType::Unit = field_type {
                quote! {#field_name}
            } else {
                quote! {#field_name(#field_type)}
            }
        },
    );
    let maybe_disable_std = no_std.then(|| quote! {#[NoStd]});

    let used_generics: HashSet<Ident> = components
        .iter()
        .flat_map(|component| component.field_type.generics())
        .filter_map(|generic_type| {
            if let GenericType::Named(name) = generic_type {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    let phantom_types = generics
        .iter()
        .filter(|generic| !used_generics.contains(generic))
        .map(|generic| {
            quote! {::core::marker::PhantomData<#generic>}
        })
        .collect_vec();

    let extra_variants = (!phantom_types.is_empty()).then(|| {
        quote! {
            #[Ignore]
            IgnoreMe(#(#phantom_types),*)
        }
    });

    quote! {
        #[allow(clippy::enum_variant_names)]
        #[derive(
            Clone,
            Debug,
            Eq,
            PartialEq,
            ::fuels::macros::Parameterize,
            ::fuels::macros::Tokenizable,
            ::fuels::macros::TryFrom
        )]
        #maybe_disable_std
        pub enum #enum_ident <#(#generics: ::fuels::core::traits::Tokenizable + ::fuels::core::traits::Parameterize),*> {
            #(#enum_variants,)*
            #extra_variants
        }
    }
}
